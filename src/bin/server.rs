use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use homework_module_two_streaming_quotes::protocol;
use homework_module_two_streaming_quotes::tickers;

/// Порт, на котором будет слушать TCP-сервер
const SERVER_PORT: u16 = 7878;

/// Путь к файлу с тикерами
const TICKERS_FILE_PATH: &str = "assets/tickers.txt";

fn main() -> std::io::Result<()> {
    let tickers = match tickers::read_tickers_from_file(TICKERS_FILE_PATH) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to read tickers file '{}': {}", TICKERS_FILE_PATH, e);
            std::process::exit(1);
        }
    };

    let tickers_set: HashSet<String> = tickers.into_iter().map(|ticker| ticker.0).collect();
    let tickers_set = Arc::new(tickers_set);

    println!(
        "Loaded {} tickers from {}",
        tickers_set.len(),
        TICKERS_FILE_PATH
    );

    let listener = TcpListener::bind(("127.0.0.1", SERVER_PORT))?;
    println!("Server listening on 127.0.0.1:{}", SERVER_PORT);
    println!("Press Ctrl+C to stop");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tickers_ref = Arc::clone(&tickers_set);

                thread::spawn(move || {
                    handle_client(stream, tickers_ref);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}

/// Обрабатывает одного клиента в отдельном потоке
fn handle_client(mut stream: TcpStream, available_tickers: Arc<HashSet<String>>) {
    let client_addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to get peer address: {}", e);
            return;
        }
    };

    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();

    if let Err(e) = reader.read_line(&mut line) {
        eprintln!("Client {}: Failed to read line: {}", client_addr, e);
        return;
    }

    let trimmed_line = line.trim_end_matches('\n').trim_end_matches('\r');
    println!("Client {}: Received: '{}'", client_addr, trimmed_line);

    match protocol::parse_stream_command(&line, &available_tickers) {
        Ok(command) => {
            let response = protocol::ok_response();

            if let Err(e) = stream.write_all(response.as_bytes()) {
                eprintln!("Client {}: Failed to send OK: {}", client_addr, e);
                return;
            }

            println!(
                "Client {}: Subscription OK - {} tickers -> {}",
                client_addr,
                command.tickers.len(),
                command.udp_addr
            );

            println!(
                "Client {}: Would subscribe to: {:?}",
                client_addr,
                command
                    .tickers
                    .iter()
                    .map(|t| t.0.as_str())
                    .collect::<Vec<&str>>()
            );
        }
        Err(e) => {
            let response = protocol::error_to_response(&e);
            if let Err(e) = stream.write_all(response.as_bytes()) {
                eprintln!(
                    "Client {}: Failed to send error response: {}",
                    client_addr, e
                );
                return;
            }

            println!("Client {}: Subscription rejected: {}", client_addr, e);
        }
    }

    println!("Client {}: Connection closed", client_addr);
}
