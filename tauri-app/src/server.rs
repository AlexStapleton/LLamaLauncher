//! One owner for the server process, its worker tree and private bridge snapshot.
use std::{
    path::PathBuf,
    process::Child,
    sync::{Arc, Mutex},
};

pub struct Snapshot(pub PathBuf);
impl Drop for Snapshot {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
pub struct TemporaryProcess {
    child: Child,
    tree: crate::process_tree::ProcessTree,
}
impl TemporaryProcess {
    pub fn new(mut child: Child) -> Result<Self, String> {
        match crate::process_tree::ProcessTree::attach(&child) {
            Ok(tree) => Ok(Self { child, tree }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }
}
impl std::ops::Deref for TemporaryProcess {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.child
    }
}
impl std::ops::DerefMut for TemporaryProcess {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}
impl Drop for TemporaryProcess {
    fn drop(&mut self) {
        self.tree.terminate(&mut self.child);
        let _ = self.child.wait();
    }
}

pub struct Instance {
    pub config: crate::config::Config,
    pub child: Child,
    pub tree: crate::process_tree::ProcessTree,
    pub _snapshot: Option<Snapshot>,
    pub endpoint: String,
    pub _logs: Option<crate::logging::Relays>,
    pub api_token: String,
    pub api_prefix: String,
}
impl Drop for Instance {
    fn drop(&mut self) {
        self.tree.terminate(&mut self.child);
        let _ = self.child.wait();
        self._logs.take();
    }
}

#[derive(Default)]
pub struct Manager {
    pub benchmark: bool,
    pub benchmark_server: Option<State>,
    pub isolated: bool,
    pub instance: Option<Instance>,
    pub phase: String,
    pub generation: u64,
    pub shutdown: bool,
}
#[derive(Default, Clone)]
pub struct State(pub Arc<Mutex<Manager>>);

pub fn stop(state: &State) -> Result<(), String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.phase = "stopping".into();
    // Serialize cleanup with start; only background commands acquire this lock.
    manager.instance.take();
    manager.generation = manager.generation.wrapping_add(1);
    manager.phase = "stopped".into();
    Ok(())
}

pub fn status(state: &State) -> Result<String, String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    if manager.benchmark {
        return Ok("benchmarking".into());
    }
    if let Some(instance) = manager.instance.as_mut() {
        if let Some(exit) = instance.child.try_wait().map_err(|e| e.to_string())? {
            manager.instance.take();
            manager.phase = format!("failed ({exit})");
        }
    }
    Ok(if manager.phase.is_empty() {
        "stopped".into()
    } else {
        manager.phase.clone()
    })
}

pub fn check_readiness(state: &State, client: &reqwest::blocking::Client) {
    let Ok(session) = crate::session::current(state) else {
        return;
    };
    let generation = session.generation;
    let ready = probe_endpoint(&session.endpoint, client, &session.prefix, &session.token);
    let Ok(mut manager) = state.0.lock() else {
        return;
    };
    if generation == manager.generation && manager.instance.is_some() {
        manager.phase = if ready { "running" } else { "loading" }.into();
    }
}
fn probe_endpoint(
    endpoint: &str,
    client: &reqwest::blocking::Client,
    prefix: &str,
    token: &str,
) -> bool {
    #[cfg(unix)]
    if let Some(socket) = endpoint.strip_prefix("unix:") {
        use std::io::Write;
        let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket) else {
            return false;
        };
        let timeout = Some(std::time::Duration::from_millis(800));
        if stream.set_read_timeout(timeout).is_err() || stream.set_write_timeout(timeout).is_err() {
            return false;
        }
        if stream
            .write_all(format!("GET {prefix}/health HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n").as_bytes())
            .is_err()
        {
            return false;
        }
        let mut line = String::new();
        let mut reader = std::io::BufReader::new(stream);
        return crate::framing::read_line(&mut reader, &mut line, 1024).is_ok()
            && line.split_whitespace().nth(1) == Some("200");
    }
    client
        .get(format!("{endpoint}{prefix}/health"))
        .bearer_auth(token)
        .send()
        .is_ok_and(|r| r.status().is_success())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        process::{Command, Stdio},
    };
    #[test]
    fn readiness_and_stop_clean_up_owned_process_and_private_snapshot() {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "tests::process_lifecycle_fixture", "--ignored"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn().unwrap();
        let tree = crate::process_tree::ProcessTree::attach(&child).unwrap();
        let snapshot = std::env::temp_dir().join(format!("llama-snapshot-{}.bin", child.id()));
        std::fs::write(&snapshot, b"fixture").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            assert!(stream.read(&mut request).unwrap() > 0);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .unwrap();
        });
        let state = State::default();
        {
            let mut manager = state.0.lock().unwrap();
            manager.phase = "loading".into();
            manager.instance = Some(Instance {
                config: crate::config::Config::default(),
                child,
                tree,
                _snapshot: Some(Snapshot(snapshot.clone())),
                _logs: None,
                api_token: String::new(),
                api_prefix: String::new(),
                endpoint,
            });
        }
        assert_eq!(status(&state).unwrap(), "loading");
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .unwrap();
        check_readiness(&state, &client);
        assert_eq!(status(&state).unwrap(), "running");
        worker.join().unwrap();
        let other = state.clone();
        std::thread::spawn(move || stop(&other).unwrap())
            .join()
            .unwrap();
        assert_eq!(status(&state).unwrap(), "stopped");
        assert!(!snapshot.exists());
        stop(&state).unwrap();
    }
    #[test]
    fn failed_launch_scope_removes_snapshot() {
        let path =
            std::env::temp_dir().join(format!("llama-failed-snapshot-{}.bin", std::process::id()));
        std::fs::write(&path, b"fixture").unwrap();
        let result: Result<(), String> = {
            let _snapshot = Snapshot(path.clone());
            Err("spawn failed".into())
        };
        assert!(result.is_err());
        assert!(!path.exists());
    }
    #[cfg(unix)]
    #[test]
    fn probes_health_over_a_unix_socket() {
        let socket = std::env::temp_dir().join(format!("llama-health-{}.sock", std::process::id()));
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            assert!(stream.read(&mut request).unwrap() > 0);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap();
        });
        let client = reqwest::blocking::Client::new();
        assert!(probe_endpoint(
            &format!("unix:{}", socket.display()),
            &client,
            "",
            ""
        ));
        worker.join().unwrap();
        std::fs::remove_file(socket).unwrap();
    }
}
