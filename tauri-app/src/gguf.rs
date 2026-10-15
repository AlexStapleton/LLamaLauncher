//! Read metadata only, with limits before allocation and checked tensor arithmetic.
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
#[derive(Clone, Serialize)]
pub struct ModelMetadata {
    pub path: String,
    pub architecture: String,
    pub name: String,
    pub parameters: u64,
    pub bytes: u64,
    pub context: u64,
    pub quantization: String,
    pub metadata: Value,
    pub warnings: Vec<String>,
    pub estimate_low: u64,
    pub estimate_high: u64,
    pub resource_estimate: Value,
}
struct Reader {
    file: File,
    remaining: u64,
}
impl Reader {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        if self.remaining < N as u64 {
            return Err("GGUF metadata exceeds 32 MiB".into());
        }
        self.remaining -= N as u64;
        let mut b = [0; N];
        self.file.read_exact(&mut b).map_err(|_| "Truncated GGUF")?;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.u64()?;
        if n > 1024 * 1024 || n > self.remaining {
            return Err("GGUF metadata exceeds limits".into());
        }
        self.remaining -= n;
        let mut b = vec![0; n as usize];
        self.file
            .read_exact(&mut b)
            .map_err(|_| "Truncated GGUF string")?;
        String::from_utf8(b).map_err(|_| "Invalid GGUF text".into())
    }
    fn value(&mut self, t: u32, depth: u32) -> Result<Value, String> {
        if depth > 2 {
            return Err("Nested GGUF array exceeds limit".into());
        }
        Ok(match t {
            0 => json!(self.bytes::<1>()?[0]),
            1 => json!(self.bytes::<1>()?[0] as i8),
            2 => json!(u16::from_le_bytes(self.bytes()?)),
            3 => json!(i16::from_le_bytes(self.bytes()?)),
            4 => json!(self.u32()?),
            5 => json!(i32::from_le_bytes(self.bytes()?)),
            6 => json!(f32::from_le_bytes(self.bytes()?)),
            7 => json!(self.bytes::<1>()?[0] != 0),
            8 => json!(self.string()?),
            10 => json!(self.u64()?),
            11 => json!(i64::from_le_bytes(self.bytes()?)),
            12 => json!(f64::from_le_bytes(self.bytes()?)),
            9 => {
                let t = self.u32()?;
                let n = self.u64()?;
                if n > 1_000_000 {
                    return Err("GGUF array too large".into());
                }
                for _ in 0..n {
                    self.value(t, depth + 1)?;
                }
                json!({"array_length":n})
            }
            _ => return Err("Unsupported GGUF metadata type".into()),
        })
    }
}
pub fn inspect(path: &Path, context: u64, parallel: u64) -> Result<ModelMetadata, String> {
    inspect_cached(path, context, parallel)
}
fn inspect_cached(path: &Path, context: u64, parallel: u64) -> Result<ModelMetadata, String> {
    type Cache = std::sync::Mutex<
        std::collections::HashMap<
            (std::path::PathBuf, u64, std::time::SystemTime, u64, u64),
            ModelMetadata,
        >,
    >;
    static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
    let info = std::fs::metadata(path).map_err(|_| "Cannot access GGUF file")?;
    let key = (
        path.to_owned(),
        info.len(),
        info.modified()
            .map_err(|_| "Cannot inspect file identity")?,
        context,
        parallel,
    );
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(v.clone());
    }
    let result = read_metadata(path, context, parallel)?;
    // Split groups may change independently of the selected first shard.
    if result
        .metadata
        .get("split.count")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        <= 1
    {
        let mut cache = cache.lock().map_err(|e| e.to_string())?;
        if cache.len() >= 32 {
            cache.clear();
        }
        cache.insert(key, result.clone());
    }
    Ok(result)
}
fn read_metadata(path: &Path, context: u64, parallel: u64) -> Result<ModelMetadata, String> {
    let file = File::open(path).map_err(|_| "Cannot open model")?;
    let mut bytes = file.metadata().map_err(|e| e.to_string())?.len();
    let mut r = Reader {
        file,
        remaining: 32 * 1024 * 1024,
    };
    if r.bytes::<4>()? != *b"GGUF" {
        return Err("Not a GGUF model".into());
    }
    if !matches!(r.u32()?, 2 | 3) {
        return Err("Unsupported GGUF version".into());
    }
    let tensors = r.u64()?;
    let fields = r.u64()?;
    if tensors > 1_000_000 || fields > 100_000 {
        return Err("GGUF header exceeds limits".into());
    }
    let mut meta = BTreeMap::new();
    for _ in 0..fields {
        let k = r.string()?;
        let t = r.u32()?;
        let v = r.value(t, 0)?;
        if !k.starts_with("tokenizer.") {
            meta.insert(k, v);
        }
    }
    let mut params = 0u64;
    let mut types = BTreeMap::<u32, u64>::new();
    for _ in 0..tensors {
        r.string()?;
        let dims = r.u32()?;
        if !(1..=4).contains(&dims) {
            return Err("Invalid tensor dimensions".into());
        }
        let mut n = 1u64;
        for _ in 0..dims {
            n = n.checked_mul(r.u64()?).ok_or("Tensor size overflow")?;
        }
        params = params.checked_add(n).ok_or("Parameter count overflow")?;
        let t = r.u32()?;
        *types.entry(t).or_default() += 1;
        r.u64()?;
    }
    let position = r.file.stream_position().map_err(|e| e.to_string())?;
    if position > bytes {
        return Err("Truncated GGUF tensors".into());
    }
    r.file
        .seek(SeekFrom::Start(position))
        .map_err(|e| e.to_string())?;
    let architecture = meta
        .get("general.architecture")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    let number = |suffix: &str| {
        meta.get(&format!("{architecture}.{suffix}"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let trained = number("context_length");
    let layers = number("block_count");
    let embed = number("embedding_length");
    let heads = number("attention.head_count");
    let kvheads = number("attention.head_count_kv");
    let mut warnings = Vec::new();
    let split = meta.get("split.count").and_then(Value::as_u64).unwrap_or(1);
    if split > 1 {
        if split > 1024 {
            return Err("GGUF shard count exceeds 1024".into());
        }
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("Invalid shard filename")?;
        let (stem, _) = name
            .rsplit_once("-of-")
            .ok_or("Split GGUF needs its standard shard filenames")?;
        let (prefix, index) = stem.rsplit_once('-').ok_or("Invalid shard filename")?;
        if index != "00001" {
            return Err("Select the first GGUF shard".into());
        }
        let parent = path.parent().ok_or("Missing shard folder")?;
        params = 0;
        for i in 1..=split {
            let sibling = parent.join(format!("{prefix}-{i:05}-of-{split:05}.gguf"));
            let info = std::fs::metadata(&sibling)
                .map_err(|_| format!("Missing GGUF shard {i} of {split}"))?;
            if i > 1 {
                bytes = bytes.checked_add(info.len()).ok_or("Shard size overflow")?;
            }
        }
        warnings.push("File size includes all shards. Parameter count unavailable without inspecting every shard.".into());
    }
    if matches!(
        architecture.as_str(),
        "flux" | "stable-diffusion" | "sd1" | "sd2" | "sdxl" | "clip"
    ) {
        warnings.push("This appears to be a diffusion model or companion, not a standalone llama-server text model.".into());
    }
    let key_length = match number("attention.key_length") {
        0 if heads > 0 => embed / heads,
        n => n,
    };
    let value_length = match number("attention.value_length") {
        0 if heads > 0 => embed / heads,
        n => n,
    };
    let kv = if heads > 0 && kvheads > 0 {
        layers
            .checked_mul(key_length.saturating_add(value_length))
            .and_then(|v| v.checked_mul(kvheads))
            .and_then(|v| v.checked_mul(context.clamp(1, 1_000_000)))
            .and_then(|v| v.checked_mul(parallel.clamp(1, 1024)))
            .and_then(|v| v.checked_mul(2))
            .unwrap_or(0)
    } else {
        0
    };
    if kv == 0 {
        warnings.push("KV cache estimate unavailable for this architecture.".into());
    }
    warnings.push("Estimate uses FP16 KV cache and a buffer allowance; GPU split, backend and actual cache precision change allocation.".into());
    if trained > 0 && context > trained {
        warnings.push("Requested context exceeds the model's trained context.".into());
    }
    let quant = |t: u32| match t {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        6 => "Q5_0",
        7 => "Q5_1",
        8 => "Q8_0",
        10 => "Q2_K",
        11 => "Q3_K",
        12 => "Q4_K",
        13 => "Q5_K",
        14 => "Q6_K",
        30 => "BF16",
        _ => "Other",
    };
    Ok(ModelMetadata {
        path: path.to_string_lossy().into(),
        architecture,
        name: meta
            .get("general.name")
            .and_then(Value::as_str)
            .unwrap_or("Unnamed")
            .into(),
        parameters: params,
        bytes,
        context: trained,
        quantization: types
            .keys()
            .map(|t| quant(*t))
            .collect::<Vec<_>>()
            .join(" / "),
        metadata: json!(meta),
        warnings,
        estimate_low: if kv == 0 {
            0
        } else {
            bytes.saturating_add(kv).saturating_add(256 * 1024 * 1024)
        },
        estimate_high: if kv == 0 {
            0
        } else {
            bytes
                .saturating_add(kv)
                .saturating_add(bytes / 4)
                .saturating_add(1024 * 1024 * 1024)
        },
        resource_estimate: Value::Null,
    })
}

pub fn inspect_config(config: &crate::config::Config) -> Result<ModelMetadata, String> {
    let path = std::path::Path::new(&config.model_file);
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::path::Path::new(&config.models_path).join(path)
    };
    let context = crate::session::option(config, &["--ctx-size", "-c"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096);
    let parallel = crate::session::option(config, &["--parallel", "-np"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let mut model = inspect(&path, context, parallel)?;
    let precision = |name: &str| match name {
        "f32" => Some(4.0),
        "f16" | "bf16" => Some(2.0),
        "q8_0" => Some(1.0625),
        "q4_0" | "iq4_nl" => Some(0.5625),
        "q4_1" => Some(0.625),
        "q5_0" => Some(0.6875),
        "q5_1" => Some(0.75),
        _ => None,
    };
    let k = crate::session::option(config, &["--cache-type-k", "-ctk"]).unwrap_or("f16".into());
    let v = crate::session::option(config, &["--cache-type-v", "-ctv"]).unwrap_or("f16".into());
    if let (Some(kbytes), Some(vbytes)) = (precision(&k), precision(&v)) {
        if model.estimate_low > 0 {
            let kv = model
                .estimate_low
                .saturating_sub(model.bytes)
                .saturating_sub(256 * 1024 * 1024);
            let adjusted = (kv as f64 * (kbytes + vbytes) / 4.0) as u64;
            model.estimate_low = model
                .estimate_low
                .saturating_sub(kv)
                .saturating_add(adjusted);
            model.estimate_high = model
                .estimate_high
                .saturating_sub(kv)
                .saturating_add(adjusted);
            model
                .warnings
                .retain(|w| !w.starts_with("Estimate uses FP16"));
            model.warnings.push(format!("Estimate uses {k} / {v} KV cache. Buffer allowance and GPU placement are approximate; runtime fitting remains authoritative."));
            let layer_count = model
                .metadata
                .get(format!("{}.block_count", model.architecture))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let setting =
                crate::session::option(config, &["--gpu-layers", "--n-gpu-layers", "-ngl"]);
            let offload = setting.as_ref().and_then(|s| {
                if s == "all" || s == "-1" {
                    Some(layer_count)
                } else {
                    s.parse::<u64>().ok()
                }
            });
            if let Some(layers) = offload.filter(|_| layer_count > 0) {
                let fraction =
                    (layers.min(layer_count) as f64 / layer_count as f64).clamp(0.0, 1.0);
                let gpu_weights = (model.bytes as f64 * fraction) as u64;
                let gpu_kv = (adjusted as f64 * fraction) as u64;
                let gpu = gpu_weights.saturating_add(gpu_kv);
                model.resource_estimate = json!({"ram_low":model.bytes.saturating_sub(gpu_weights).saturating_add(adjusted-gpu_kv).saturating_add(256*1024*1024),"ram_high":model.estimate_high,"vram_low":gpu,"vram_high":if gpu>0{gpu.saturating_add(gpu/4).saturating_add(512*1024*1024)}else{0},"cache_k":k,"cache_v":v,"assumption":"Aggregate GPU allocation; proportional layer estimate. Output/embedding tensors, mmap residency, KV offload settings and backend buffers may change placement."});
            }
        }
    } else {
        model.estimate_low = 0;
        model.estimate_high = 0;
        model
            .warnings
            .push("Unknown KV precision; memory estimate unavailable.".into());
    }
    Ok(model)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_short_and_excessive_headers() {
        let p = std::env::temp_dir().join(format!("gguf-test-{}", std::process::id()));
        std::fs::write(&p, b"GGUF").unwrap();
        assert!(inspect(&p, 4096, 1).is_err());
        let mut b = b"GGUF".to_vec();
        b.extend(3u32.to_le_bytes());
        b.extend(u64::MAX.to_le_bytes());
        b.extend(0u64.to_le_bytes());
        std::fs::write(&p, b).unwrap();
        assert!(inspect(&p, 4096, 1).is_err());
        std::fs::remove_file(p).unwrap();
    }
    #[test]
    fn parses_metadata_without_loading_weights_and_invalidates_cache() {
        fn text(b: &mut Vec<u8>, s: &str) {
            b.extend((s.len() as u64).to_le_bytes());
            b.extend(s.as_bytes());
        }
        let p = std::env::temp_dir().join(format!("gguf-valid-{}", std::process::id()));
        let mut b = b"GGUF".to_vec();
        b.extend(3u32.to_le_bytes());
        b.extend(1u64.to_le_bytes());
        b.extend(6u64.to_le_bytes());
        text(&mut b, "general.architecture");
        b.extend(8u32.to_le_bytes());
        text(&mut b, "llama");
        for (key, value) in [
            ("llama.context_length", 4096),
            ("llama.block_count", 32),
            ("llama.embedding_length", 4096),
            ("llama.attention.head_count", 32),
            ("llama.attention.head_count_kv", 8),
        ] {
            text(&mut b, key);
            b.extend(4u32.to_le_bytes());
            b.extend((value as u32).to_le_bytes());
        }
        text(&mut b, "weight");
        b.extend(2u32.to_le_bytes());
        b.extend(2u64.to_le_bytes());
        b.extend(4u64.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend(0u64.to_le_bytes());
        fs_write(&p, &b);
        let m = inspect(&p, 4096, 1).unwrap();
        assert_eq!(m.architecture, "llama");
        assert_eq!(m.parameters, 8);
        assert_eq!(m.context, 4096);
        assert!(m.estimate_low > m.bytes);
        assert!(inspect(&p, 8192, 1).unwrap().estimate_low > m.estimate_low);
        fs_write(&p, b"not GGUF");
        assert!(inspect(&p, 4096, 1).is_err());
        std::fs::remove_file(p).unwrap();
        fn fs_write(p: &Path, b: &[u8]) {
            std::fs::write(p, b).unwrap();
        }
    }
}
