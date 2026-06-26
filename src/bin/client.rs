use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::thread;
use std::time::Duration;

use homework_module_two_streaming_quotes::quote::StockQuote;
use homework_module_two_streaming_quotes::tickers;

/// Интервал отправки PING (2 секунды)
const PING_INTERVAL: Duration = Duration::from_secs(2);

/// Таймаут для UDP приёма (100 мс)
const UDP_RECV_TIMEOUT: Duration = Duration::from_millis(100);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <SERVER_ADDR> <UDP_PORT> <TICKERS_FILE>", args[0]);
        eprintln!(
            "Example: {} 127.0.0.1:7878 9000 assets/tickers.txt",
            args[0]
        );
        std::process::exit(1);
    }

    let server_addr = &args[1];
    let udp_port: u16 = args[2].parse().expect("Invalid UDP port");
    let tickers_file = &args[3];

    println!("Starting client...");
    println!("Server: {}", server_addr);
    println!("UDP port: {}", udp_port);
    println!("Tickers file: {}", tickers_file);

    let tickers = tickers::read_tickers_from_file(tickers_file)?;
    if tickers.is_empty() {
        eprintln!("No tickers found in file: {}", tickers_file);
        std::process::exit(1);
    }

    let tickers_str: Vec<String> = tickers.iter().map(|t| t.0.clone()).collect();
    let tickers_list = tickers_str.join(",");
    println!("Subscribing to: {}", tickers_list);

    let bind_addr = format!("0.0.0.0:{}", udp_port);
    let udp_socket = UdpSocket::bind(&bind_addr)?;
    udp_socket.set_read_timeout(Some(UDP_RECV_TIMEOUT))?;
    println!("Listening for quotes on {}", bind_addr);

    let temp_tcp = TcpStream::connect(server_addr)?;
    let local_addr = temp_tcp.local_addr()?;

    let local_ip = local_addr.ip();
    let udp_addr_for_server = SocketAddr::new(local_ip, udp_port);

    println!("Our UDP address for server: {}", udp_addr_for_server);

    drop(temp_tcp);

    let mut tcp_stream = TcpStream::connect(server_addr)?;
    tcp_stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    println!("Connected to server: {}", server_addr);

    let stream_command = format!("STREAM {} {}\n", udp_addr_for_server, tickers_list);
    tcp_stream.write_all(stream_command.as_bytes())?;
    println!("Sent: {}", stream_command.trim());

    let mut reader = BufReader::new(&tcp_stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    if response.trim() != "OK" {
        eprintln!("Server error: {}", response.trim());
        std::process::exit(1);
    }
    println!("Server response: {}", response.trim());

    let ping_socket = udp_socket.try_clone()?;
    let ping_handle = thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let mut server_addr: Option<SocketAddr> = None;

        while server_addr.is_none() {
            match ping_socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    let data = String::from_utf8_lossy(&buf[..size]);
                    if data.contains('|') {
                        println!("Got first quote from server, sending PING to {}", src_addr);
                        server_addr = Some(src_addr);
                    }
                }
                Err(_) => continue,
            }
        }

        let server_addr = server_addr.unwrap();

        loop {
            if let Err(e) = ping_socket.send_to(b"PING\n", server_addr) {
                eprintln!("Failed to send PING: {}", e);
            }
            thread::sleep(PING_INTERVAL);
        }
    });

    println!("Waiting for quotes...");
    println!("Press Ctrl+C to stop\n");

    let mut buf = [0u8; 2048];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, src_addr)) => {
                let data = String::from_utf8_lossy(&buf[..size]);
                let line = data.trim_end();

                match StockQuote::from_wire_line(line) {
                    Ok(quote) => {
                        println!(
                            "[{}] {} | {:.2} | {} | {}",
                            src_addr,
                            quote.ticker.0,
                            quote.price.0,
                            quote.volume.0,
                            quote.timestamp_ms.0
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to parse quote: {} (data: {})", e, line);
                    }
                }
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::TimedOut {
                    eprintln!("UDP receive error: {}", e);
                    break;
                }
            }
        }
    }

    ping_handle.join().unwrap_or_else(|e| {
        eprintln!("PING thread panicked: {:?}", e);
    });

    Ok(())
}
