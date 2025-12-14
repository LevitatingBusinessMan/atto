use std::{io::{self, BufRead, BufReader, Read, Write}, os::fd::{AsRawFd, BorrowedFd}, process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio}, sync::mpsc::{self, Receiver, Sender, channel}, thread};

use anyhow::Context;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use serde_json::json;
use tracing::{error, info, trace};

pub struct LspConnection {
    child: Child,
    stdin: ChildStdin,
    stderr: ChildStderr,
    initialized: bool,
    stdout_rx: Receiver<anyhow::Result<serde_json::Value>>,
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

        let (tx, rx) = channel::<anyhow::Result<serde_json::Value>>();
        
        thread::spawn(move || Self::read_thread(tx, BufReader::new(stdout)));
        
        let mut me = Self {
            child,
            stdin,
            stderr,
            initialized: false,
            stdout_rx: rx,
        };
        
        me.initialize().context("failed to initialize lsp")?;
        
        Ok(me)
    }
    
    /* NOTE
     * It might be best to read stdout using a thread, but poll for stderr.
     */
     
    /// loop for reading the stdout
    fn read_thread(tx: Sender<anyhow::Result<serde_json::Value>>, mut stdout: BufReader<ChildStdout>) {
        loop {
            match Self::read_stdout(&mut stdout) {
                Ok(json) => {
                    tx.send(Ok(json)).unwrap();
                },
                Err(e) => {
                    error!("lsp read error {e:?}");
                    tx.send(Err(e)).unwrap();
                    break;
                },
            }
        }
    }
    
    fn read_stdout(stdout: &mut BufReader<ChildStdout>) -> anyhow::Result<serde_json::Value> {
        let mut line = String::new();
        stdout.read_line(&mut line)?;
        trace!("lsp stdout {line:?}");
        let length = line.strip_prefix("Content-Length: ").context("invalid lsp response")?;
        let length: usize = length.trim_end().parse::<usize>().context("invalid lsp response")?;
        stdout.read_line(&mut String::new())?;
        let mut out = String::with_capacity(length);
        stdout.by_ref().take(length as u64).read_to_string(&mut out)?;
        trace!("lsp stdout {out}");
        let json = serde_json::from_str(&out)?;
        Ok(json)
    }
    
    fn poll_stderr(&mut self) -> anyhow::Result<bool> {
        let mut pollfds = [
          PollFd::new(unsafe { BorrowedFd::borrow_raw(self.stderr.as_raw_fd()) }, PollFlags::POLLIN)  
        ];
        if poll(&mut pollfds, PollTimeout::ZERO)? > 0 {
            Ok(true)
        } else {
            Ok(false)            
        }
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
        self.stdout_rx.recv()??;

        // send notification
        let json = json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }).to_string();
        self.write(json)?;

        self.initialized = true;
        
        Ok(())
    }
    
    fn write(&mut self, json: String) -> io::Result<()> {
        self.stdin.write_fmt(format_args!("Content-Length: {}\r\n\r\n", json.len()))?;
        self.stdin.write_all(json.as_bytes())?;
        self.stdin.flush()?;
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
        self.stdout_rx.recv()??;
        Ok(())
    }
    
    pub fn read_stderr(&mut self) -> io::Result<String> {
        let mut string = String::new();
        self.stderr.read_to_string(&mut string)?;
        Ok(string)
    }
    
}
