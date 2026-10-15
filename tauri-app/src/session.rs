//! Only talks to a server owned by this app; carries its effective auth and prefix.
use crate::{config::Config, server};
use serde_json::{json, Value};
use std::{sync::atomic::AtomicBool, time::Duration};
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
#[derive(Clone)]
pub struct Session {
    pub endpoint: String,
    pub prefix: String,
    pub token: String,
    pub pid: u32,
    pub router: bool,
    pub generation: u64,
}
pub fn option(config: &Config, names: &[&str]) -> Option<String> {
    let mut value = config
        .argument_overrides
        .iter()
        .rev()
        .find(|a| names.contains(&a.flag.as_str()))
        .and_then(|a| a.values.first())
        .cloned();
    let mut args = config.additional_args.iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(a, b)| (a, Some(b)));
        if names.contains(&flag) {
            value = inline.map(str::to_owned).or_else(|| args.next().cloned());
        }
    }
    value
}
pub fn authenticate(config: &Config, isolated: bool) -> Result<(String, String), String> {
    let mut token = option(config, &["--api-key"])
        .or_else(|| {
            if isolated {
                None
            } else {
                std::env::var("LLAMA_API_KEY").ok()
            }
        })
        .unwrap_or_default();
    if let Some(path) = option(config, &["--api-key-file"]).or_else(|| {
        if isolated {
            None
        } else {
            std::env::var("LLAMA_ARG_API_KEY_FILE").ok()
        }
    }) {
        use std::io::Read;
        let path = std::path::Path::new(&path);
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            std::path::Path::new(&config.llama_cpp_path)
                .parent()
                .ok_or("Missing runtime directory")?
                .join(path)
        };
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "Cannot read API key file")?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read API key file")?;
        if bytes.len() > 65536 {
            return Err("API key file too large".into());
        }
        let keys = String::from_utf8(bytes).map_err(|_| "Invalid API key file")?;
        token = keys
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .unwrap_or("")
            .to_owned();
    }
    token = token.split(',').next().unwrap_or("").trim().into();
    if token.contains(['\r', '\n']) {
        return Err("Invalid API authentication header".into());
    }
    let prefix = option(config, &["--api-prefix"])
        .or_else(|| {
            if isolated {
                None
            } else {
                std::env::var("LLAMA_ARG_API_PREFIX").ok()
            }
        })
        .unwrap_or_default();
    if prefix.contains(['\r', '\n', '?', '#']) || (!prefix.is_empty() && !prefix.starts_with('/')) {
        return Err("Invalid API prefix".into());
    }
    Ok((token, prefix.trim_end_matches('/').into()))
}
pub fn current(state: &server::State) -> Result<Session, String> {
    let m = state.0.lock().map_err(|_| "Server state unavailable")?;
    let i = m.instance.as_ref().ok_or("Start a server first")?;
    Ok(Session {
        endpoint: i.endpoint.clone(),
        prefix: i.api_prefix.clone(),
        token: i.api_token.clone(),
        pid: i.child.id(),
        router: i.config.router_mode,
        generation: m.generation,
    })
}
pub enum Body {
    Http(reqwest::Response),
    #[cfg(unix)]
    Unix {
        reader: tokio::io::BufReader<tokio::net::UnixStream>,
        chunked: bool,
        left: usize,
        done: bool,
    },
}
#[cfg(unix)]
async fn bounded_line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    line: &mut String,
    limit: usize,
) -> Result<usize, String> {
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf().await.map_err(|_| "Socket read failed")?;
        if available.is_empty() {
            return Err("Truncated socket headers".into());
        }
        let n = available
            .iter()
            .position(|b| *b == b'\n')
            .map_or(available.len(), |i| i + 1);
        if bytes.len() + n > limit {
            return Err("Oversized socket header".into());
        }
        let done = available[n - 1] == b'\n';
        bytes.extend_from_slice(&available[..n]);
        reader.consume(n);
        if done {
            *line = String::from_utf8(bytes).map_err(|_| "Invalid socket text")?;
            return Ok(line.len());
        }
    }
}
impl Body {
    pub async fn chunk(&mut self) -> Result<Option<Vec<u8>>, String> {
        match self {
            Self::Http(r) => r
                .chunk()
                .await
                .map(|c| c.map(|c| c.to_vec()))
                .map_err(|_| "Server connection interrupted".into()),
            #[cfg(unix)]
            Self::Unix {
                reader,
                chunked,
                left,
                done,
            } => {
                if *done {
                    return Ok(None);
                }
                if *chunked && *left == 0 {
                    let mut line = String::new();
                    let n = bounded_line(reader, &mut line, 1024).await?;
                    if n > 1024 {
                        return Err("Oversized HTTP chunk".into());
                    }
                    *left = usize::from_str_radix(line.trim().split(';').next().unwrap_or(""), 16)
                        .map_err(|_| "Invalid HTTP chunk")?;
                    if *left == 0 {
                        *done = true;
                        return Ok(None);
                    }
                }
                let mut bytes = vec![0; if *chunked { (*left).min(8192) } else { 8192 }];
                let n = reader
                    .read(&mut bytes)
                    .await
                    .map_err(|_| "Socket read failed")?;
                if n == 0 {
                    *done = true;
                    return Ok(None);
                }
                bytes.truncate(n);
                if *chunked {
                    *left -= n;
                    if *left == 0 {
                        let mut crlf = [0; 2];
                        reader
                            .read_exact(&mut crlf)
                            .await
                            .map_err(|_| "Truncated chunk")?;
                        if crlf != *b"\r\n" {
                            return Err("Invalid chunk framing".into());
                        }
                    }
                }
                Ok(Some(bytes))
            }
        }
    }
}
pub async fn request(
    s: &Session,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Body, String> {
    if path.contains(['\r', '\n']) {
        return Err("Invalid API path".into());
    }
    let path = format!("{}{path}", s.prefix);
    #[cfg(unix)]
    if let Some(socket) = s.endpoint.strip_prefix("unix:") {
        let mut stream = tokio::net::UnixStream::connect(socket)
            .await
            .map_err(|_| "Cannot connect to server socket")?;
        let data = body.map(|v| v.to_string()).unwrap_or_default();
        let auth = if s.token.is_empty() {
            String::new()
        } else {
            format!("Authorization: Bearer {}\r\n", s.token)
        };
        if auth.contains('\n') && !auth.ends_with("\r\n") {
            return Err("Invalid API credentials".into());
        }
        stream.write_all(format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{data}",data.len()).as_bytes()).await.map_err(|_|"Socket write failed")?;
        let mut reader = tokio::io::BufReader::new(stream);
        let mut line = String::new();
        bounded_line(&mut reader, &mut line, 1024).await?;
        let status = line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or("Invalid socket HTTP response")?;
        let mut size = 0;
        let mut chunked = false;
        loop {
            line.clear();
            let n = bounded_line(&mut reader, &mut line, 4096).await?;
            size += n;
            if size > 16384 || n == 0 {
                return Err("Invalid HTTP headers".into());
            }
            if line == "\r\n" {
                break;
            }
            if line.to_ascii_lowercase().starts_with("transfer-encoding:")
                && line.contains("chunked")
            {
                chunked = true;
            }
        }
        if !(200..300).contains(&status) {
            return Err(format!(
                "Server returned HTTP {status}. Check API authentication and runtime capabilities."
            ));
        }
        return Ok(Body::Unix {
            reader,
            chunked,
            left: 0,
            done: false,
        });
    }
    // A pooled client avoids TLS/connection setup on every metrics/model poll.
    static CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> =
        std::sync::OnceLock::new();
    let client = CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| "Cannot create server client".into())
        })
        .as_ref()
        .map_err(Clone::clone)?;
    let mut r = client.request(
        reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| "Invalid HTTP method")?,
        format!("{}{path}", s.endpoint),
    );
    if !s.token.is_empty() {
        r = r.bearer_auth(&s.token);
    }
    if let Some(body) = body {
        r = r.json(&body);
    }
    let response = r
        .send()
        .await
        .map_err(|_| "Cannot reach server. Check its endpoint and trusted TLS certificate.")?;
    if !response.status().is_success() {
        return Err(format!(
            "Server returned HTTP {}. Check API authentication and runtime capabilities.",
            response.status().as_u16()
        ));
    }
    Ok(Body::Http(response))
}
pub async fn json_request(
    s: &Session,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut response = request(s, method, path, body).await?;
        let mut data = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if data.len() + chunk.len() > 4 * 1024 * 1024 {
                return Err("Server response exceeded 4 MiB".into());
            }
            data.extend(chunk);
        }
        serde_json::from_slice(&data).map_err(|_| "Server returned invalid JSON".into())
    })
    .await
    .map_err(|_| "Server request timed out")?
}
pub async fn completion(
    s: &Session,
    prompt: &str,
    model: &str,
    max: u32,
    cancel: &AtomicBool,
    mut delta: impl FnMut(&str),
) -> Result<Value, String> {
    let started = std::time::Instant::now();
    let mut first = None;
    let mut text = String::new();
    let mut usage = Value::Null;
    let mut buffer = Vec::new();
    let mut chunks = 0u64;
    let body = json!({"model":model,"messages":[{"role":"user","content":prompt}],"max_tokens":max,"temperature":0,"stream":true,"stream_options":{"include_usage":true},"tool_choice":"none"});
    let mut response = tokio::select! {r=tokio::time::timeout(Duration::from_secs(120),request(s,"POST","/v1/chat/completions",Some(body)))=>r.map_err(|_|"Inference connection timed out")??,_=cancelled(cancel)=>return Err("Cancelled".into())};
    'stream: loop {
        let chunk = tokio::select! {r=tokio::time::timeout(Duration::from_secs(120),response.chunk())=>r.map_err(|_|"Inference timed out")??,_=cancelled(cancel)=>return Err("Cancelled".into())};
        let Some(chunk) = chunk else {
            return Err("Inference stream ended before its completion marker".into());
        };
        if buffer.len().saturating_add(chunk.len()) > 1024 * 1024 {
            return Err("Inference event exceeds 1 MiB".into());
        }
        buffer.extend(chunk);
        while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<_> = buffer.drain(..=pos).collect();
            let line = std::str::from_utf8(&line)
                .map_err(|_| "Invalid inference text")?
                .trim();
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break 'stream;
            }
            let v: Value = serde_json::from_str(data).map_err(|_| "Invalid inference event")?;
            if let Some(error) = v.get("error") {
                return Err(format!(
                    "Inference rejected: {}",
                    error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("server error")
                ));
            }
            if let Some(t) = v
                .pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
            {
                if !t.is_empty() {
                    first.get_or_insert(started.elapsed().as_secs_f64());
                    chunks += 1;
                    text.push_str(t);
                    if text.len() > 1024 * 1024 {
                        return Err("Inference output exceeded 1 MiB".into());
                    }
                    delta(t);
                }
            }
            if v.get("usage").is_some_and(|v| !v.is_null()) {
                usage = v["usage"].clone();
            }
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    let tokens = usage.get("completion_tokens").and_then(Value::as_u64);
    Ok(
        json!({"text":text,"first_token_seconds":first,"seconds":elapsed,"tokens":tokens,"tokens_per_second":tokens.map(|n|n as f64/(elapsed-first.unwrap_or(0.0)).max(0.001)),"usage":usage,"stream_chunks":chunks}),
    )
}
pub async fn cancelled(cancel: &AtomicBool) {
    loop {
        if crate::jobs::check(cancel).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Router event support is probed by opening its advertised endpoint. Older
/// runtimes fall back to the UI's bounded five-second model-state poll.
pub fn watch_router(app: tauri::AppHandle, state: server::State) {
    use tauri::Manager;
    tauri::async_runtime::spawn(async move {
        loop {
            if state.0.lock().map_or(true, |m| m.shutdown) {
                break;
            }
            let session = current(&state).ok().filter(|s| s.router);
            if let Some(s) = session {
                let stale = async {
                    loop {
                        if state
                            .0
                            .lock()
                            .map_or(true, |m| m.shutdown || m.generation != s.generation)
                        {
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                };
                let stream = async {
                    let mut body = tokio::time::timeout(
                        Duration::from_secs(5),
                        request(&s, "GET", "/models/sse", None),
                    )
                    .await
                    .map_err(|_| "Router event timeout")??;
                    let _ = app.emit_all("router-events", json!({"connected":true}));
                    let mut buffer = Vec::new();
                    while let Some(chunk) =
                        fresh_router_chunk(&mut body, Duration::from_secs(20)).await?
                    {
                        if buffer.len() + chunk.len() > 256 * 1024 {
                            return Err("Router event exceeded 256 KiB".into());
                        }
                        buffer.extend(chunk);
                        while let Some(pos) = buffer.iter().position(|b| *b == b'\n') {
                            let line: Vec<_> = buffer.drain(..=pos).collect();
                            let line = std::str::from_utf8(&line)
                                .map_err(|_| "Invalid router event")?
                                .trim();
                            if let Some(data) = line.strip_prefix("data:") {
                                if let Ok(event) = serde_json::from_str::<Value>(data.trim()) {
                                    let _=app.emit_all("router-change",json!({"model":event["model"],"event":event["event"],"status":event["data"]["status"]}));
                                }
                            }
                        }
                    }
                    Ok::<(), String>(())
                };
                tokio::select! {_=stale=>{},_=stream=>{}}
                let _ = app.emit_all("router-events", json!({"connected":false}));
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
async fn fresh_router_chunk(body: &mut Body, limit: Duration) -> Result<Option<Vec<u8>>, String> {
    tokio::time::timeout(limit, body.chunk())
        .await
        .map_err(|_| "Router events became stale")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn completion_fixture(body: &str, keep_open: bool) -> Result<Value, String> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let body = body.to_string();
        let worker = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 8192];
            assert!(socket.read(&mut buf).await.unwrap() > 0);
            if keep_open {
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{}\r\n", body.len(), body).as_bytes()).await.unwrap();
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else {
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
            }
        });
        let s = Session {
            endpoint,
            prefix: String::new(),
            token: String::new(),
            pid: 0,
            router: false,
            generation: 0,
        };
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            completion(&s, "test", "fixture", 16, &AtomicBool::new(false), |_| {}),
        )
        .await;
        worker.abort();
        result.expect("Completion must not wait for HTTP EOF after DONE")
    }
    #[tokio::test]
    async fn done_finishes_even_when_http_stream_remains_open() {
        let result = completion_fixture(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Complete\"}}]}\n\ndata: [DONE]\n\n",
            true,
        )
        .await
        .unwrap();
        assert_eq!(result["text"], "Complete");
    }
    #[tokio::test]
    async fn an_open_stalled_router_stream_becomes_stale() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let worker = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut data = [0; 4096];
            let _ = socket.read(&mut data).await.unwrap();
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 100\r\n\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        let response = reqwest::Client::new().get(endpoint).send().await.unwrap();
        let mut body = Body::Http(response);
        assert!(fresh_router_chunk(&mut body, Duration::from_millis(50))
            .await
            .unwrap_err()
            .contains("stale"));
        worker.abort();
    }
    #[tokio::test]
    async fn truncated_stream_is_reported_as_a_failure() {
        assert!(completion_fixture(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Partial\"}}]}\n\n",
            false
        )
        .await
        .unwrap_err()
        .contains("completion marker"));
    }
    #[tokio::test]
    async fn streaming_uses_prefix_auth_and_disables_tools() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let worker = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                request.extend(&buf[..n]);
                if let Some(pos) = request.windows(4).position(|b| b == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..pos]).to_ascii_lowercase();
                    let size = headers
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if request.len() >= pos + 4 + size {
                        assert!(headers.starts_with("post /prefix/v1/chat/completions "));
                        assert!(headers.contains("authorization: bearer fixture-token"));
                        let body: Value = serde_json::from_slice(&request[pos + 4..]).unwrap();
                        assert_eq!(body["tool_choice"], "none");
                        assert!(body.get("tools").is_none());
                        break;
                    }
                }
            }
            let body="data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\ndata: {\"usage\":{\"completion_tokens\":2}}\n\ndata: [DONE]\n\n";
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).await.unwrap();
            for chunk in body.as_bytes().chunks(7) {
                socket.write_all(chunk).await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        let s = Session {
            endpoint,
            prefix: "/prefix".into(),
            token: "fixture-token".into(),
            pid: 0,
            router: false,
            generation: 0,
        };
        let mut deltas = String::new();
        let result = completion(&s, "Test", "fixture", 128, &AtomicBool::new(false), |t| {
            deltas.push_str(t)
        })
        .await
        .unwrap();
        assert_eq!(deltas, "Hello world");
        assert_eq!(result["tokens"], 2);
        assert!(result["first_token_seconds"].is_number());
        worker.await.unwrap();
    }
    #[tokio::test]
    async fn cancelling_a_stalled_stream_returns_without_waiting_for_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let worker = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = [0; 8192];
            assert!(s.read(&mut buf).await.unwrap() > 0);
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let c = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let s = Session {
            endpoint,
            prefix: String::new(),
            token: String::new(),
            pid: 0,
            router: false,
            generation: 0,
        };
        let start = std::time::Instant::now();
        assert!(completion(&s, "Test", "", 128, &cancel, |_| {})
            .await
            .unwrap_err()
            .contains("Cancelled"));
        assert!(start.elapsed() < Duration::from_secs(1));
        worker.abort();
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn socket_headers_are_bounded_before_allocation() {
        let mut reader = tokio::io::BufReader::new(&b"xxxxxxxxxxxxxxxxxxxx"[..]);
        let mut line = String::new();
        assert!(bounded_line(&mut reader, &mut line, 8).await.is_err());
    }
}
