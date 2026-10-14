pub use crate::storage::{load_profile, read_profile, save_profile, write_profile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{BufReader, Read, Write},
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub transport: String,
    pub url: String,
    pub bearer_token: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: String,
    pub enabled: bool,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "docker_gateway".into(),
            transport: "http".into(),
            url: "http://localhost:9811/mcp".into(),
            bearer_token: String::new(),
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: String::new(),
            enabled: false,
        }
    }
}
impl std::fmt::Debug for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpProfile")
            .field("name", &self.name)
            .field("transport", &self.transport)
            .field("credentials", &"[redacted]")
            .finish()
    }
}
#[derive(Clone, Default, Serialize)]
pub struct Report {
    pub connected: bool,
    pub server: String,
    pub endpoint: String,
    pub protocol: String,
    pub tools: Vec<Value>,
    pub message: String,
}

pub struct HttpSession {
    client: reqwest::blocking::Client,
    pub profile: Profile,
    session: Option<String>,
    protocol: String,
    tools_changed: bool,
}
impl Drop for HttpSession {
    fn drop(&mut self) {
        if let Some(session) = &self.session {
            let mut request = self
                .client
                .delete(&self.profile.url)
                .header("Mcp-Session-Id", session)
                .header("MCP-Protocol-Version", &self.protocol)
                .timeout(Duration::from_secs(2));
            if !self.profile.bearer_token.is_empty() {
                request = request.bearer_auth(&self.profile.bearer_token);
            }
            let _ = request.send();
        }
    }
}
impl HttpSession {
    pub fn new(profile: Profile) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Cannot create HTTP client.".to_string())?;
        Ok(Self {
            client,
            profile,
            session: None,
            protocol: "2025-11-25".into(),
            tools_changed: false,
        })
    }
    pub fn exchange(&mut self, message: &Value) -> Result<Option<Value>, String> {
        let mut response = None;
        for _ in 0..4 {
            let mut request = self
                .client
                .post(&self.profile.url)
                .header("Accept", "application/json, text/event-stream")
                .json(message);
            if !self.profile.bearer_token.is_empty() {
                request = request.bearer_auth(&self.profile.bearer_token);
            }
            if let Some(session) = &self.session {
                request = request.header("Mcp-Session-Id", session);
            }
            if message["method"] != "initialize" {
                request = request.header("MCP-Protocol-Version", &self.protocol);
            }
            let reply = request.send().map_err(|_| {
                "Could not reach MCP endpoint (network error or timeout).".to_string()
            })?;
            if reply.status().is_redirection() {
                let location = reply
                    .headers()
                    .get("location")
                    .and_then(|h| h.to_str().ok())
                    .ok_or("Redirect has no Location header.")?;
                let current =
                    reqwest::Url::parse(&self.profile.url).map_err(|_| "Invalid MCP URL.")?;
                let next = current
                    .join(location)
                    .map_err(|_| "Invalid redirect URL.")?;
                if next.origin() != current.origin() {
                    return Err("MCP endpoint redirects to a different origin. Use its final URL explicitly.".into());
                }
                self.profile.url = next.to_string();
                continue;
            }
            if !reply.status().is_success() {
                return Err(format!(
                    "MCP endpoint returned HTTP {}.{}",
                    reply.status().as_u16(),
                    if reply.status().as_u16() == 401 {
                        " Check the bearer token."
                    } else {
                        ""
                    }
                ));
            }
            if let Some(session) = reply
                .headers()
                .get("mcp-session-id")
                .and_then(|s| s.to_str().ok())
            {
                self.session = Some(session.into());
            }
            response = Some(reply);
            break;
        }
        let response = response.ok_or("Too many MCP redirects.")?;
        let Some(id) = message
            .get("id")
            .filter(|_| message.get("method").is_some())
        else {
            return Ok(None);
        };
        let event_stream = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("text/event-stream");
        let reply = if event_stream {
            let mut reader = BufReader::new(response);
            let mut line = String::new();
            let mut data = Vec::new();
            let mut total = 0usize;
            loop {
                line.clear();
                let size = crate::framing::read_line(
                    &mut reader,
                    &mut line,
                    crate::framing::MESSAGE_LIMIT,
                )
                .map_err(|_| "MCP event stream failed or timed out.")?;
                if size == 0 {
                    return Err("MCP event stream closed before the response.".into());
                }
                total += size;
                if total > 4 * 1024 * 1024 {
                    return Err("MCP response exceeded 4 MiB.".into());
                }
                let line = line.trim_end_matches(['\r', '\n']);
                if let Some(value) = line.strip_prefix("data:") {
                    data.push(value.trim_start().to_string());
                }
                if line.is_empty() && !data.is_empty() {
                    let value: Value = serde_json::from_str(&data.join("\n"))
                        .map_err(|_| "Invalid MCP event data.")?;
                    data.clear();
                    if value.get("method").is_some() {
                        if value["method"] == "notifications/tools/list_changed" {
                            self.tools_changed = true;
                        }
                        if let Some(reply) = server_request_reply(&value) {
                            self.exchange(&reply)?;
                        }
                        continue;
                    }
                    if value.get("id") == Some(id) {
                        break value;
                    }
                }
            }
        } else {
            let mut text = String::new();
            response
                .take(4 * 1024 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|_| "Could not read MCP response.")?;
            if text.len() > 4 * 1024 * 1024 {
                return Err("MCP response exceeded 4 MiB.".into());
            }
            let value: Value = serde_json::from_str(&text)
                .map_err(|_| "Endpoint did not return MCP JSON. Check its /mcp URL.".to_string())?;
            if value.get("id") != Some(id) {
                return Err("MCP returned an unexpected response ID.".into());
            }
            value
        };
        if message["method"] == "initialize" {
            if let Some(protocol) = reply["result"]["protocolVersion"].as_str() {
                self.protocol = protocol.into();
            }
        }
        Ok(Some(reply))
    }
}
struct StdioSession {
    child: Child,
    stdin: ChildStdin,
    replies: mpsc::Receiver<Value>,
    _tree: crate::process_tree::ProcessTree,
    tools_changed: bool,
}
impl Drop for StdioSession {
    fn drop(&mut self) {
        self._tree.terminate(&mut self.child);
        let _ = self.child.wait();
    }
}
enum Transport {
    Http(HttpSession),
    Stdio(StdioSession),
}
pub struct Client {
    transport: Transport,
    next_id: u64,
}
impl Client {
    pub fn new(profile: Profile) -> Result<Self, String> {
        let transport = if profile.transport == "http" {
            Transport::Http(HttpSession::new(profile)?)
        } else {
            let mut command = Command::new(&profile.command);
            command
                .args(&profile.args)
                .envs(&profile.env)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            if !profile.cwd.is_empty() {
                command.current_dir(&profile.cwd);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                command.process_group(0);
            }
            let mut child = command.spawn().map_err(|_| {
                "Could not start MCP executable. Use an executable path and separate arguments."
                    .to_string()
            })?;
            let tree = match crate::process_tree::ProcessTree::attach(&child) {
                Ok(tree) => tree,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error);
                }
            };
            let stdin = child.stdin.take().ok_or("No MCP stdin pipe.")?;
            let stdout = child.stdout.take().ok_or("No MCP stdout pipe.")?;
            let (tx, rx) = mpsc::sync_channel(8);
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();
                while matches!(crate::framing::read_line(&mut reader, &mut line, crate::framing::MESSAGE_LIMIT), Ok(size) if size > 0)
                {
                    if let Ok(value) = serde_json::from_str::<Value>(&line) {
                        // Fail closed on a flood rather than block a reader indefinitely.
                        if tx.try_send(value).is_err() {
                            break;
                        }
                    }
                }
            });
            Transport::Stdio(StdioSession {
                child,
                stdin,
                replies: rx,
                _tree: tree,
                tools_changed: false,
            })
        };
        Ok(Self {
            transport,
            next_id: 1,
        })
    }
    fn exchange(&mut self, message: Value) -> Result<Option<Value>, String> {
        match &mut self.transport {
            Transport::Http(http) => http.exchange(&message),
            Transport::Stdio(stdio) => {
                let line =
                    serde_json::to_string(&message).map_err(|_| "Could not encode MCP request.")?;
                writeln!(stdio.stdin, "{}", line)
                    .and_then(|_| stdio.stdin.flush())
                    .map_err(|_| "MCP process stopped or closed its input.".to_string())?;
                let Some(id) = message.get("id") else {
                    return Ok(None);
                };
                let deadline = Instant::now() + Duration::from_secs(15);
                loop {
                    let response = stdio
                        .replies
                        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                        .map_err(|_| {
                            "MCP process did not respond within 15 seconds.".to_string()
                        })?;
                    if response.get("method").is_some() {
                        if response["method"] == "notifications/tools/list_changed" {
                            stdio.tools_changed = true;
                        }
                        if let Some(reply) = server_request_reply(&response) {
                            writeln!(stdio.stdin, "{reply}")
                                .and_then(|_| stdio.stdin.flush())
                                .map_err(|_| "MCP process input closed.")?;
                        }
                        continue;
                    }
                    if response.get("id") == Some(id) {
                        return Ok(Some(response));
                    }
                }
            }
        }
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let reply = self
            .exchange(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?
            .ok_or("Missing MCP response.")?;
        if let Some(error) = reply.get("error") {
            return Err(format!(
                "MCP {} failed (JSON-RPC {}).",
                method, error["code"]
            ));
        }
        reply
            .get("result")
            .cloned()
            .ok_or_else(|| "MCP response has no result.".into())
    }
    pub fn initialize(&mut self) -> Result<Report, String> {
        let result=self.call("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"llama-server-configurator","version":"1.0.0"}}))?;
        let protocol = result["protocolVersion"]
            .as_str()
            .ok_or("MCP initialization did not negotiate a protocol version.")?
            .to_owned();
        if !["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"].contains(&protocol.as_str()) {
            return Err("MCP server selected an unsupported protocol version.".into());
        }
        self.exchange(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))?;
        let tools = self.list_tools()?;
        let endpoint = match &self.transport {
            Transport::Http(http) => http.profile.url.clone(),
            Transport::Stdio(_) => "Local stdio process".into(),
        };
        Ok(Report {
            connected: true,
            server: result["serverInfo"]["name"]
                .as_str()
                .unwrap_or("MCP server")
                .into(),
            endpoint,
            protocol,
            tools,
            message: "Connection initialized and tools discovered.".into(),
        })
    }
    fn list_tools(&mut self) -> Result<Vec<Value>, String> {
        let mut tools = Vec::new();
        let mut cursor = None;
        let mut seen = std::collections::HashSet::new();
        let mut catalog_bytes = 0usize;
        for _ in 0..100 {
            let result = self.call(
                "tools/list",
                cursor
                    .as_ref()
                    .map(|c| json!({"cursor":c}))
                    .unwrap_or_else(|| json!({})),
            )?;
            let page = result["tools"]
                .as_array()
                .ok_or("MCP tools/list response has no tools array.")?;
            catalog_bytes += serde_json::to_vec(page)
                .map_err(|_| "Invalid tool catalog.")?
                .len();
            if catalog_bytes > crate::framing::CATALOG_LIMIT {
                return Err("MCP tool catalog exceeded 16 MiB.".into());
            }
            tools.extend(page.iter().cloned());
            if tools.len() > 10000 {
                return Err("MCP server returned more than 10,000 tools.".into());
            }
            cursor = result["nextCursor"].as_str().map(str::to_owned);
            if let Some(next) = &cursor {
                if !seen.insert(next.clone()) {
                    return Err("MCP server repeated a pagination cursor.".into());
                }
            } else {
                break;
            }
        }
        if cursor.is_some() {
            return Err("MCP tool pagination exceeded 100 pages.".into());
        }
        Ok(tools)
    }
    pub fn refresh_tools_if_changed(&mut self) -> Result<Option<Vec<Value>>, String> {
        let changed = match &mut self.transport {
            Transport::Http(http) => std::mem::take(&mut http.tools_changed),
            Transport::Stdio(stdio) => std::mem::take(&mut stdio.tools_changed),
        };
        if changed {
            self.list_tools().map(Some)
        } else {
            Ok(None)
        }
    }
    pub fn ping(&mut self) -> Result<(), String> {
        self.call("ping", json!({})).map(|_| ())
    }
}
fn server_request_reply(message: &Value) -> Option<Value> {
    let id = message.get("id")?;
    Some(if message["method"] == "ping" {
        json!({"jsonrpc":"2.0","id":id,"result":{}})
    } else {
        json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Client capability not supported"}})
    })
}
#[derive(Default)]
pub struct Connection {
    pub client: Option<Client>,
    pub report: Report,
    pub generation: u64,
}
#[derive(Serialize)]
pub struct Heartbeat {
    pub connected: bool,
    pub message: String,
}
#[derive(Default)]
pub struct Manager {
    pub connection: Arc<Mutex<Connection>>,
}

pub fn server_config(profile: &Profile, bridge_snapshot: &Path) -> Result<String, String> {
    let (command, args) = if profile.transport == "http" {
        (
            std::env::current_exe()
                .map_err(|_| "Cannot locate MCP bridge executable.")?
                .to_string_lossy()
                .into_owned(),
            vec![
                "--mcp-http-bridge".to_owned(),
                bridge_snapshot.to_string_lossy().into_owned(),
            ],
        )
    } else {
        (profile.command.clone(), profile.args.clone())
    };
    let mut entry = json!({"command":command,"args":args,"env":profile.env,"timeout_ms":30000});
    if !profile.cwd.is_empty() {
        entry["cwd"] = json!(profile.cwd);
    }
    let mut servers = serde_json::Map::new();
    servers.insert(profile.name.clone(), entry);
    serde_json::to_string(&json!({"mcpServers":servers}))
        .map_err(|_| "Cannot encode llama-server MCP configuration.".into())
}
pub fn bridge(path: &Path) -> Result<(), String> {
    let profile = read_profile(path)?;
    let mut http = HttpSession::new(profile)?;
    let stdin = std::io::stdin();
    let mut output = std::io::stdout().lock();
    let mut reader = stdin.lock();
    let mut line = String::new();
    while crate::framing::read_line(&mut reader, &mut line, crate::framing::MESSAGE_LIMIT)
        .map_err(|_| "MCP bridge frame exceeded limits or input failed.")?
        > 0
    {
        let mut message: Value =
            serde_json::from_str(&line).map_err(|_| "Invalid MCP bridge JSON.")?;
        if message["method"] == "initialize" {
            // This bridge cannot service sampling, roots or elicitation requests.
            message["params"]["capabilities"] = json!({});
        }
        let response = match http.exchange(&message) {
            Ok(response) => response,
            Err(error) => message
                .get("id")
                .map(|id| json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":error}})),
        };
        if let Some(response) = response {
            writeln!(output, "{}", response)
                .and_then(|_| output.flush())
                .map_err(|_| "MCP bridge output closed.".to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::BufRead};
    #[test]
    fn server_requests_cannot_be_confused_with_client_replies() {
        let reply = server_request_reply(
            &json!({"jsonrpc":"2.0","id":1,"method":"sampling/createMessage"}),
        )
        .unwrap();
        assert_eq!(reply["error"]["code"], -32601);
        assert!(
            server_request_reply(&json!({"method":"notifications/tools/list_changed"})).is_none()
        );
        assert_eq!(
            server_request_reply(&json!({"id":1,"method":"ping"})).unwrap()["result"],
            json!({})
        );
    }
    #[test]
    fn protected_profile_roundtrip_and_bridge_config_omit_credentials() {
        let folder = std::env::temp_dir().join(format!("llama-mcp-profile-{}", std::process::id()));
        let path = folder.join("profile.bin");
        let profile = Profile {
            bearer_token: "fixture-only-token".into(),
            enabled: true,
            ..Profile::default()
        };
        write_profile(&path, &profile).unwrap();
        assert_eq!(
            read_profile(&path).unwrap().bearer_token,
            profile.bearer_token
        );
        #[cfg(windows)]
        assert!(!fs::read(&path)
            .unwrap()
            .windows(profile.bearer_token.len())
            .any(|part| part == profile.bearer_token.as_bytes()));
        let config = server_config(&profile, &path).unwrap();
        assert!(!config.contains(&profile.bearer_token));
        let value: Value = serde_json::from_str(&config).unwrap();
        let entry = &value["mcpServers"][&profile.name];
        assert_eq!(entry["args"][0], "--mcp-http-bridge");
        assert!(entry.get("cwd").is_none());
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn http_handshake_session_headers_sse_pagination_and_ping() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            for step in 0..6 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    header.push(byte[0]);
                    assert!(header.len() < 10000);
                }
                let header = String::from_utf8(header).unwrap();
                assert!(header
                    .to_lowercase()
                    .contains("authorization: bearer fixture-token"));
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; length];
                stream.read_exact(&mut body).unwrap();
                if step == 5 {
                    assert!(header.starts_with("DELETE"));
                    write!(
                        stream,
                        "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .unwrap();
                    continue;
                }
                let request: Value = serde_json::from_slice(&body).unwrap();
                if step > 0 {
                    assert!(header
                        .to_lowercase()
                        .contains("mcp-session-id: fixture-session"));
                    assert!(header
                        .to_lowercase()
                        .contains("mcp-protocol-version: 2025-11-25"));
                }
                if step == 1 {
                    assert_eq!(request["method"], "notifications/initialized");
                    write!(
                        stream,
                        "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .unwrap();
                    continue;
                }
                let result = match step {
                    0 => {
                        json!({"protocolVersion":"2025-11-25","serverInfo":{"name":"Fixture Gateway"},"capabilities":{"tools":{}}})
                    }
                    2 => {
                        json!({"tools":[{"name":"first_tool","inputSchema":{"type":"object"}}],"nextCursor":"page2"})
                    }
                    3 => {
                        assert_eq!(request["params"]["cursor"], "page2");
                        json!({"tools":[{"name":"second_tool","inputSchema":{"type":"object"}}]})
                    }
                    4 => {
                        assert_eq!(request["method"], "ping");
                        json!({})
                    }
                    _ => unreachable!(),
                };
                let reply = json!({"jsonrpc":"2.0","id":request["id"],"result":result});
                let (kind, body) = if step == 2 {
                    (
                        "text/event-stream",
                        format!("event: message\ndata: {}\n\n", reply),
                    )
                } else {
                    ("application/json", reply.to_string())
                };
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: {}\r\nMcp-Session-Id: fixture-session\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",kind,body.len(),body).unwrap();
            }
        });
        let profile = Profile {
            url: format!("http://{}/mcp", address),
            bearer_token: "fixture-token".into(),
            ..Profile::default()
        };
        let mut client = Client::new(profile).unwrap();
        let report = client.initialize().unwrap();
        assert_eq!(report.server, "Fixture Gateway");
        assert_eq!(report.tools.len(), 2);
        client.ping().unwrap();
        drop(client);
        worker.join().unwrap();
    }
    #[test]
    fn stdio_handshake_and_tools_are_read_without_running_tools() {
        let executable = std::env::current_exe().unwrap();
        let profile = Profile {
            transport: "stdio".into(),
            command: executable.to_string_lossy().into_owned(),
            args: vec![
                "--exact".into(),
                "mcp::tests::stdio_fixture".into(),
                "--ignored".into(),
                "--nocapture".into(),
            ],
            ..Profile::default()
        };
        let mut client = Client::new(profile).unwrap();
        let report = client.initialize().unwrap();
        assert_eq!(report.tools.len(), 1);
        client.ping().unwrap();
    }
    #[test]
    #[ignore = "MCP fixture launched only by the stdio client test"]
    fn stdio_fixture() {
        for line in std::io::stdin().lock().lines() {
            let request: Value = serde_json::from_str(&line.unwrap()).unwrap();
            let Some(id) = request.get("id") else {
                continue;
            };
            let result = match request["method"].as_str().unwrap() {
                "initialize" => {
                    json!({"protocolVersion":"2025-11-25","serverInfo":{"name":"Fixture Stdio"},"capabilities":{"tools":{}}})
                }
                "tools/list" => {
                    json!({"tools":[{"name":"fixture_tool","inputSchema":{"type":"object"}}]})
                }
                "ping" => json!({}),
                method => panic!("Unexpected method: {}", method),
            };
            println!("{}", json!({"jsonrpc":"2.0","id":id,"result":result}));
            std::io::stdout().flush().unwrap();
        }
    }
}
