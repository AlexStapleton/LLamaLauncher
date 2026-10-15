//! Drain both child streams independently, with bounded private rotating files.
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    thread::JoinHandle,
};
pub const STREAM_LIMIT: u64 = 10 * 1024 * 1024;
const BACKUPS: usize = 2;
pub struct Writer {
    file: Option<fs::File>,
    path: PathBuf,
    size: u64,
    limit: u64,
}
impl Writer {
    pub fn open(path: &Path, limit: u64) -> std::io::Result<Self> {
        fs::create_dir_all(path.parent().unwrap())?;
        let mut result = Self {
            file: None,
            path: path.into(),
            size: 0,
            limit,
        };
        result.rotate()?;
        Ok(result)
    }
    fn rotate(&mut self) -> std::io::Result<()> {
        self.file.take();
        for i in (1..=BACKUPS).rev() {
            let from = if i == 1 {
                self.path.clone()
            } else {
                self.backup(i - 1)
            };
            let to = self.backup(i);
            match fs::remove_file(&to) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            if from.exists() {
                fs::rename(from, to)?;
            }
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        self.file = Some(options.open(&self.path)?);
        self.size = 0;
        Ok(())
    }
    fn backup(&self, n: usize) -> PathBuf {
        self.path.with_file_name(format!(
            "{}.{}",
            self.path.file_name().unwrap().to_string_lossy(),
            n
        ))
    }
    fn append(&mut self, mut bytes: &[u8]) -> std::io::Result<()> {
        while !bytes.is_empty() {
            if self.size >= self.limit {
                self.rotate()?;
            }
            let count = bytes.len().min((self.limit - self.size) as usize);
            self.file.as_mut().unwrap().write_all(&bytes[..count])?;
            self.size += count as u64;
            bytes = &bytes[count..];
        }
        Ok(())
    }
}
pub struct Relays(Vec<JoinHandle<()>>);
impl Relays {
    pub fn start(
        stdout: impl Read + Send + 'static,
        stderr: impl Read + Send + 'static,
        out: Writer,
        err: Writer,
        secrets: Vec<String>,
    ) -> std::io::Result<Self> {
        fn relay(
            reader: impl Read + Send + 'static,
            mut writer: Writer,
            secrets: Vec<String>,
        ) -> std::io::Result<JoinHandle<()>> {
            std::thread::Builder::new()
                .name("server-log".into())
                .spawn(move || {
                    let mut reader = reader;
                    let mut buffer = [0; 8192];
                    let mut pending = Vec::new();
                    let mut writable = true;
                    loop {
                        let n = match reader.read(&mut buffer) {
                            Ok(n) => n,
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                            Err(_) => break,
                        };
                        pending.extend_from_slice(&buffer[..n]);
                        // Keep a bounded overlap so credentials split across pipe reads are scrubbed.
                        let overlap = secrets.iter().map(String::len).max().unwrap_or(0);
                        let mut end = if n == 0 {
                            pending.len()
                        } else {
                            pending.len().saturating_sub(overlap)
                        };
                        for secret in &secrets {
                            if secret.is_empty() {
                                continue;
                            }
                            for (start, bytes) in pending.windows(secret.len()).enumerate() {
                                if bytes == secret.as_bytes()
                                    && start < end
                                    && start + secret.len() > end
                                {
                                    end = start;
                                    break;
                                }
                            }
                        }
                        while end > 0 && end < pending.len() && pending[end] & 0xc0 == 0x80 {
                            end -= 1;
                        }
                        if end > 0 {
                            let mut text = String::from_utf8_lossy(&pending[..end]).into_owned();
                            for secret in &secrets {
                                if !secret.is_empty() {
                                    text = text.replace(secret, "[redacted]");
                                }
                            }
                            if writable && writer.append(text.as_bytes()).is_err() {
                                writable = false;
                            }
                            pending.drain(..end);
                        }
                        if n == 0 {
                            break;
                        }
                    }
                    // A disk failure must never stop draining and deadlock the server.
                })
        }
        Ok(Self(vec![
            relay(stdout, out, secrets.clone())?,
            relay(stderr, err, secrets)?,
        ]))
    }
}
impl Drop for Relays {
    fn drop(&mut self) {
        for worker in self.0.drain(..) {
            let _ = worker.join();
        }
    }
}
pub fn tail(path: &Path, limit: u64) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(limit)))?;
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_split_across_pipe_reads_are_scrubbed_and_both_streams_drain() {
        let root = std::env::temp_dir().join(format!("log-secrets-{}", std::process::id()));
        let out = root.join("stdout.log");
        let err = root.join("stderr.log");
        let secret = "fixture-secret-across-read-boundary";
        let mut data = vec![b'x'; 8180];
        data.extend_from_slice(secret.as_bytes());
        data.extend_from_slice(&vec![b'y'; 30000]);
        let relays = Relays::start(
            std::io::Cursor::new(data),
            std::io::Cursor::new(b"errors drained\n"),
            Writer::open(&out, 65536).unwrap(),
            Writer::open(&err, 65536).unwrap(),
            vec![secret.into()],
        )
        .unwrap();
        drop(relays);
        let text = fs::read_to_string(&out).unwrap();
        assert!(!text.contains(secret));
        assert!(text.contains("[redacted]"));
        assert_eq!(fs::read_to_string(&err).unwrap(), "errors drained\n");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_write_failure_does_not_stop_draining_the_pipe() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Counted {
            bytes: std::io::Cursor<Vec<u8>>,
            count: Arc<AtomicUsize>,
        }
        impl Read for Counted {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                let n = self.bytes.read(b)?;
                self.count.fetch_add(n, Ordering::Relaxed);
                Ok(n)
            }
        }
        let root = std::env::temp_dir().join(format!("log-failure-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let out = root.join("stdout.log");
        fs::write(&out, b"").unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let broken = Writer {
            file: Some(fs::File::open(&out).unwrap()),
            path: out,
            size: 0,
            limit: 65536,
        };
        let relays = Relays::start(
            Counted {
                bytes: std::io::Cursor::new(vec![b'x'; 200000]),
                count: count.clone(),
            },
            std::io::empty(),
            broken,
            Writer::open(&root.join("stderr.log"), 65536).unwrap(),
            vec![],
        )
        .unwrap();
        drop(relays);
        assert_eq!(count.load(Ordering::Relaxed), 200000);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rotation_caps_all_files_and_retains_the_latest_bytes() {
        let root = std::env::temp_dir().join(format!("bounded-logs-{}", std::process::id()));
        let path = root.join("stdout.log");
        let mut writer = Writer::open(&path, 1024).unwrap();
        writer.append(&vec![b'x'; 20000]).unwrap();
        writer.append(b"latest").unwrap();
        drop(writer);
        assert!(tail(&path, 1024).unwrap().ends_with("latest"));
        assert!(
            fs::read_dir(&root)
                .unwrap()
                .map(|e| e.unwrap().metadata().unwrap().len())
                .sum::<u64>()
                <= 3072
        );
        fs::remove_dir_all(root).unwrap();
    }
}
