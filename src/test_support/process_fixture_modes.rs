use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

pub(super) fn run_http_health() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind health fixture");
    listener
        .set_nonblocking(true)
        .expect("set health listener nonblocking");
    println!(
        "FIXTURE_HTTP {}",
        listener.local_addr().expect("health address")
    );
    std::io::stdout().flush().expect("flush health address");

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = [0_u8; 1];
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
        let _ = shutdown_tx.send(());
    });

    loop {
        if shutdown_rx.try_recv().is_ok() {
            return;
        }
        match listener.accept() {
            Ok((mut stream, _address)) => {
                let mut reader = BufReader::new(stream.try_clone().expect("clone health stream"));
                let mut request = String::new();
                reader.read_line(&mut request).expect("read health request");
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).expect("read health header");
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                }
                let status = if request.starts_with("GET /health ") {
                    "200 OK"
                } else {
                    "404 Not Found"
                };
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK"
                )
                .expect("write health response");
                stream.flush().expect("flush health response");
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("accept health request: {error}"),
        }
    }
}

pub(super) fn run_interactive_shell() {
    write_line("fixture-shell-ready");
    for line in BufReader::new(std::io::stdin()).lines() {
        write_line(&format!(
            "fixture-shell> {}",
            line.expect("read shell input")
        ));
    }
}

fn write_line(line: &str) {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}").expect("write fixture output");
    stdout.flush().expect("flush fixture output");
}
