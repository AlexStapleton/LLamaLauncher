//! Display/export boundary. Custom executable arguments are private by default.
use crate::config::Config;
pub fn secret_flag(flag: &str) -> bool {
    let name = flag.to_ascii_lowercase().replace('_', "-");
    flag == "-hft"
        || name == "auth"
        || name.ends_with("-auth")
        || name.starts_with("--auth-")
        || [
            "token",
            "password",
            "secret",
            "authorization",
            "api-key",
            "access-key",
            "credential",
            "mcp-servers",
        ]
        .iter()
        .any(|s| name.contains(s))
}
pub fn values(config: &Config) -> Vec<String> {
    let mut result = vec![config.mcp_profile.bearer_token.clone()];
    result.extend(config.mcp_profile.env.values().cloned());
    result.extend(
        config
            .mcp_profile
            .args
            .iter()
            .filter(|a| !a.starts_with('-'))
            .cloned(),
    );
    for arg in &config.mcp_profile.args {
        if let Some((flag, value)) = arg.split_once('=') {
            if secret_flag(flag) {
                result.push(value.into());
            }
        }
    }
    for item in &config.argument_overrides {
        if secret_flag(&item.flag) {
            result.extend(item.values.clone());
        }
    }
    let mut private = false;
    let mut required = false;
    for arg in &config.additional_args {
        if required {
            result.push(arg.clone());
            required = false;
            continue;
        }
        if let Some((flag, value)) = arg.split_once('=') {
            if secret_flag(flag) {
                result.push(value.into());
                private = true;
                continue;
            }
        }
        if arg.starts_with('-') {
            private = secret_flag(arg);
            required = private;
        } else if private {
            result.push(arg.clone());
        }
    }
    for (key, value) in std::env::vars() {
        if matches!(key.as_str(), "HF_TOKEN" | "LLAMA_API_KEY") {
            result.push(value);
        }
    }
    if let Some(path) = crate::session::option(config, &["--api-key-file"])
        .or_else(|| std::env::var("LLAMA_ARG_API_KEY_FILE").ok())
    {
        use std::io::Read;
        let p = std::path::Path::new(&path);
        let p = if p.is_absolute() {
            p.into()
        } else {
            std::path::Path::new(&config.llama_cpp_path)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(p)
        };
        if let Ok(file) = std::fs::File::open(p) {
            let mut bytes = Vec::new();
            if file.take(65536).read_to_end(&mut bytes).is_ok() {
                result.extend(
                    String::from_utf8_lossy(&bytes)
                        .lines()
                        .filter(|line| !line.starts_with('#'))
                        .map(|s| s.trim().to_owned()),
                );
            }
        }
    }
    for value in config
        .argument_overrides
        .iter()
        .flat_map(|a| &a.values)
        .chain(config.additional_args.iter())
        .chain(std::iter::once(&config.mcp_profile.url))
    {
        if let Ok(url) = reqwest::Url::parse(value) {
            if !url.username().is_empty()
                || url.password().is_some()
                || url.query_pairs().any(|(key, _)| secret_flag(&key))
            {
                result.push(value.clone());
            }
            if let Some(password) = url.password() {
                result.push(password.into());
            }
            for (key, value) in url.query_pairs() {
                if secret_flag(&key) {
                    result.push(value.into());
                }
            }
        }
    }
    result.retain(|s| !s.is_empty());
    result.sort_by_key(|s| std::cmp::Reverse(s.len()));
    result.dedup();
    result
}
pub fn redact(mut text: String, secrets: &[String]) -> String {
    for s in secrets {
        if !s.is_empty() {
            text = text.replace(s, "[redacted]");
        }
    }
    text
}
pub fn share(mut config: Config) -> Config {
    config.mcp_profile.enabled = false;
    config.mcp_profile.bearer_token.clear();
    config.mcp_profile.env.clear();
    config.mcp_profile.args.clear();
    config.additional_args.clear();
    config.argument_overrides.retain(|a| !secret_flag(&a.flag));
    fn public_url(value: &mut String) {
        if let Ok(mut url) = reqwest::Url::parse(value) {
            if matches!(url.scheme(), "http" | "https") {
                let _ = url.set_username("");
                let _ = url.set_password(None);
                url.set_query(None);
                url.set_fragment(None);
                *value = url.into();
            }
        }
    }
    public_url(&mut config.mcp_profile.url);
    for item in &mut config.argument_overrides {
        for value in &mut item.values {
            public_url(value);
        }
    }
    config
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_stdio_and_multiple_api_keys_are_all_known_to_log_scrubbing() {
        let mut config = Config::default();
        config.mcp_profile.args = vec!["--token=fixture-inline".into()];
        config.additional_args = vec![
            "--api-key".into(),
            "-fixture-first".into(),
            "fixture-second".into(),
            "--threads".into(),
            "4".into(),
        ];
        let known = values(&config);
        let text = redact(
            "fixture-inline -fixture-first fixture-second threads 4".into(),
            &known,
        );
        assert_eq!(text, "[redacted] [redacted] [redacted] threads 4");
    }
    #[test]
    fn public_exports_omit_arbitrary_stdio_credentials_and_authenticated_urls() {
        let mut config = Config::default();
        config.mcp_profile.args = vec!["--custom-auth".into(), "fixture-private".into()];
        config.mcp_profile.url =
            "https://user:fixture-pass@example.com/mcp?token=fixture-query".into();
        config.additional_args = vec!["unmarked-private-value".into()];
        let text = serde_json::to_string(&share(config)).unwrap();
        for value in [
            "fixture-private",
            "fixture-pass",
            "fixture-query",
            "unmarked-private-value",
        ] {
            assert!(!text.contains(value));
        }
    }
}
