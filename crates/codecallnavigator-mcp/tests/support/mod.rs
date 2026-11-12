#![allow(dead_code)]
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

pub fn engine() -> PathBuf {
    std::env::var_os("CODECALLNAVIGATOR_TEST_ENGINE")
        .or_else(|| std::env::var_os("CODECALLNAVIGATOR_ENGINE"))
        .map(PathBuf::from)
        .expect("Build the real CLI and set CODECALLNAVIGATOR_TEST_ENGINE")
}

pub fn command(engine: &Path, project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-mcp"));
    command
        .arg("--engine")
        .arg(engine)
        .arg("--project")
        .arg(project);
    command
}

pub struct Client {
    child: Child,
    input: Option<ChildStdin>,
    messages: Receiver<Result<Value, String>>,
    pending: Vec<Value>,
    next_id: u64,
    _errors: tempfile::NamedTempFile,
}

impl Client {
    pub fn launch(mut command: Command) -> Self {
        let errors = tempfile::NamedTempFile::new().unwrap();
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(errors.reopen().unwrap()))
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let message = line
                    .map_err(|e| e.to_string())
                    .and_then(|line| serde_json::from_str(&line).map_err(|e| e.to_string()));
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        let mut client = Self {
            child,
            input,
            messages,
            pending: Vec::new(),
            next_id: 1,
            _errors: errors,
        };
        let response = client.request("initialize", json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"real-cli-acceptance","version":"1"}}));
        assert!(
            response["result"]["capabilities"]["tools"].is_object(),
            "{response}"
        );
        client.notify("notifications/initialized", json!({}));
        client
    }

    pub fn start(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        id
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({"jsonrpc":"2.0","method":method,"params":params}));
    }

    fn send(&mut self, value: Value) {
        writeln!(self.input.as_mut().unwrap(), "{value}").unwrap();
        self.input.as_mut().unwrap().flush().unwrap();
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.start(method, params);
        self.wait(id)
    }

    pub fn wait(&mut self, id: u64) -> Value {
        if let Some(index) = self.pending.iter().position(|value| value["id"] == id) {
            return self.pending.remove(index);
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let wait = deadline.saturating_duration_since(Instant::now());
            let message = self
                .messages
                .recv_timeout(wait)
                .unwrap_or_else(|error| {
                    panic!(
                        "MCP response {id} unavailable: {error}; stderr={}",
                        std::fs::read_to_string(self._errors.path()).unwrap()
                    )
                })
                .unwrap();
            if message["id"] == id {
                return message;
            }
            self.pending.push(message);
        }
    }

    pub fn call(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request("tools/call", json!({"name":name,"arguments":arguments}));
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    }

    pub fn graph(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.call(name, arguments);
        assert_ne!(result["isError"], true, "{result}");
        result["structuredContent"].clone()
    }

    pub fn assert_no_response(&mut self, id: u64) {
        while let Ok(message) = self.messages.recv_timeout(Duration::from_millis(100)) {
            self.pending.push(message.unwrap());
        }
        assert!(
            self.pending.iter().all(|message| message["id"] != id),
            "The cancelled request emitted a late response: {:?}",
            self.pending
        );
    }

    pub fn disconnect(&mut self) {
        drop(self.input.take());
    }

    pub fn assert_shutdown(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "MCP disconnect failed: {}",
                    std::fs::read_to_string(self._errors.path()).unwrap()
                );
                return;
            }
            assert!(
                Instant::now() < deadline,
                "MCP did not drain owned work after disconnect"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        drop(self.input.take());
        let deadline = Instant::now() + Duration::from_secs(6);
        while self.child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

pub fn cli_query(engine: &Path, project: &Path, operation: Value) -> Value {
    let mut child = Command::new(engine)
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = json!({"version":1,"project":project,"query":operation});
    write!(child.stdin.take().unwrap(), "{request}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

pub fn typescript(root: &Path) {
    let modules = std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
        .expect("Set CODECALLNAVIGATOR_TS_NODE_MODULES to pinned TypeScript6 modules");
    std::os::unix::fs::symlink(modules, root.join("node_modules")).unwrap();
    std::fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"target":"ES2022","types":[]},"include":["*.ts"]}"#,
    )
    .unwrap();
    std::fs::write(root.join("entry.ts"), "export function leaf(): number { return 1; }\nexport function start(flag: boolean): number { if(flag) return leaf(); return 0; }\n").unwrap();
    std::fs::write(
        root.join("other.ts"),
        "export function start(): number { return 2; }\n",
    )
    .unwrap();
}
