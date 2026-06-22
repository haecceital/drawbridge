use crossterm::event;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

pub struct ProcessBridge {
    child: Child,
    pub stderr: Option<std::process::ChildStderr>,
}

impl ProcessBridge {
    pub fn from_args() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();

        let exec_idx = args.iter().position(|arg| arg == "--exec").ok_or_else(|| {
            "Error: Missing '--exec' argument. Usage: drawbridge --exec <FILE_NAME>".to_string()
        })?;

        let executable_cmd = args
            .get(exec_idx + 1)
            .ok_or_else(|| "Error: No executable file specified after '--exec'.".to_string())?;

        Self::spawn(executable_cmd)
    }

    fn spawn(executable_cmd: &str) -> Result<Self, String> {
        let args: Vec<&str> = executable_cmd.split_whitespace().collect();
        let program_path = args[0];

        let mut command = Command::new(program_path);
        if args.len() > 1 {
            command.args(&args[1..]);
        }

        command.envs(std::env::vars());

        command.env("PYTHONUNBUFFERED", "1");
        command.env("RUBY_UNBUFFERED", "1");
        command.env("_STDBUF_O", "0");
        command.env("_STDBUF_E", "0");

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start child process '{}': {}", program_path, e))?;

        let stderr = child.stderr.take();

        Ok(Self { child, stderr })
    }

    pub fn start_event_loop(&mut self) -> Result<(Receiver<(u16, u16)>, Sender<String>), String> {
        let mut child_stdin = self
            .child
            .stdin
            .take()
            .ok_or("Failed to open stdin".to_string())?;
        let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>();
        let (query_tx, query_rx) = mpsc::channel::<String>();

        thread::spawn(move || {
            loop {
                if let Ok(msg) = query_rx.try_recv() {
                    if child_stdin.write_all(msg.as_bytes()).is_err() {
                        break;
                    }
                    let _ = child_stdin.flush();
                }

                if event::poll(Duration::from_millis(10)).unwrap()
                    && let Ok(event) = crossterm::event::read()
                {
                    let msg = match event {
                        crossterm::event::Event::Key(key_event) => {
                            format!("Key|{:?}|{:?}\n", key_event.code, key_event.modifiers)
                        }
                        crossterm::event::Event::Resize(w, h) => {
                            if resize_tx.send((w, h)).is_err() {
                                break;
                            }
                            format!("Resize|{}|{}\n", w, h)
                        }
                        _ => continue,
                    };

                    if child_stdin.write_all(msg.as_bytes()).is_err() {
                        break;
                    }
                    let _ = child_stdin.flush();
                }
            }
        });

        Ok((resize_rx, query_tx))
    }

    pub fn lines(&mut self) -> impl Iterator<Item = String> {
        let stdout = self.child.stdout.take().expect("Failed to open stdout");
        BufReader::new(stdout)
            .lines()
            .map(|line| line.unwrap_or_default())
    }
}

impl Drop for ProcessBridge {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
