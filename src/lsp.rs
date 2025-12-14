use std::{io::{self, BufRead, BufReader, Read, Write}, process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio}};

use anyhow::Context;
use serde_json::json;
use tracing::{info, trace};

pub struct LspConnection {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: ChildStderr,
    initialized: bool,
}

impl LspConnection {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let mut child = Command::new(name)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
                
        let mut me = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            stderr,
            initialized: false,
        };
        
        me.initialize().context("failed to initialize lsp")?;
        
        Ok(me)
    }
        
    fn initialize(&mut self) -> anyhow::Result<()> {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "processId": self.child.id(),
                "rootPath": std::env::current_dir()?.to_str(),
                "capabilities": {
                    "textDocument": {
                        "hover": {
                            "contentFormat": ["markdown", "plaintext"]
                        }
                    }
                }
            },
            "id": 1
        }).to_string();
        
        // process request
        self.write(json)?;
        self.read()?;
        
        // send notification
        let json = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }).to_string();
        self.write(json)?;
        
        self.initialized = true;
        
        // temp sleep
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        Ok(())
    }
    
    fn write(&mut self, json: String) -> io::Result<()> {
        self.stdin.write_fmt(format_args!("Content-Length: {}\r\n\r\n", json.len()))?;
        self.stdin.write_all(json.as_bytes())?;
        self.stdin.flush()?;
        Ok(())
    }
    
    fn read(&mut self) -> anyhow::Result<()> {
        let mut line = String::new();
        self.stdout.read_line(&mut line)?;
        trace!("lsp stdout {line:?}");
        let length = line.strip_prefix("Content-Length: ").context("invalid lsp response")?;
        let length: usize = length.trim_end().parse::<usize>().context("invalid lsp response")?;
        self.stdout.read_line(&mut String::new())?;
        let mut out = String::with_capacity(length);
        self.stdout.by_ref().take(length as u64).read_to_string(&mut out)?;
        trace!("lsp stdout {out}");
        Ok(())
    }
    
    pub fn on_hover(&mut self) -> anyhow::Result<()> {
        let json = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument" : {
                    "uri": "file:///home/rein/src/atto/src/view.rs"
                },
                "position": {
                    "line": 1,
                    "character": 6
                }
            },
            "id": 2
        }).to_string();
        self.write(json)?;
        self.read()?;
        Ok(())
    }
    
    pub fn read_stderr(&mut self) -> io::Result<String> {
        let mut string = String::new();
        self.stderr.read_to_string(&mut string)?;
        Ok(string)
    }
    
}
