mod cmd;
mod display;
mod parser;
mod process;

use display::DisplayServer;
use parser::Parser;
use process::ProcessBridge;
use cmd::Cmd;

fn main() {
    let mut bridge = ProcessBridge::from_args().unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    let mut parser = Parser::new();
    let mut server = DisplayServer::new();

    let (resize_rx, query_tx) = match bridge.start_event_loop() {
        Ok(r) => r,
        Err(e) => {
            drop(server);
            drop(bridge);

            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let mut err_msg: Option<String> = None;

    for line in bridge.lines() {
        if let Ok((w, h)) = resize_rx.try_recv() {
            server.resize(w, h);
        }

        match parser.parse_line(&line) {
            Ok(cmds) => {
                for cmd in cmds {
                    match cmd {
                        Cmd::QuerySize => {
                            let (w, h) = server.get_size();
                            if query_tx.send(format!("Size|{}|{}\n", w, h)).is_err() {
                                break;
                            }
                        }
                        _ => server.execute(cmd),
                    }
                }
            }
            Err(e) => {
                err_msg = Some(e);
                break;
            }
        }
    }

    let child_stderr = bridge.stderr.take();

    drop(server);
    drop(bridge);

    if let Some(mut stderr) = child_stderr {
        let mut err_msg = String::new();
        if std::io::Read::read_to_string(&mut stderr, &mut err_msg).is_ok() && !err_msg.is_empty() {
            eprintln!("\x1b[31;1m--- Python Runtime Error Traceback ---\x1b[0m");
            eprintln!("{}", err_msg);
            eprintln!("\x1b[31;1m--------------------------------------\x1b[0m");
        }
    }

    if let Some(msg) = err_msg {
        eprintln!("{}", msg);
        std::process::exit(1);
    }

    println!("DisplayServer context processed to the end. Goodbye!");
}
