use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rand::RngExt;
use rand::rng;

use homework_module_two_streaming_quotes::protocol;
use homework_module_two_streaming_quotes::quote::{Price, StockQuote, Ticker, Timestamp, Volume};
use homework_module_two_streaming_quotes::tickers;

/// Порт, на котором будет слушать TCP-сервер
const SERVER_PORT: u16 = 7878;

/// Путь к файлу с тикерами
const TICKERS_FILE_PATH: &str = "assets/tickers.txt";

/// Интервал генерации котировок (100 мс)
const GENERATION_INTERVAL: Duration = Duration::from_millis(100);

/// Тайм-аут без PING (5 секунд)
const PING_TIMEOUT: Duration = Duration::from_secs(5);

/// Буфер для UDP приёма (2 KiB)
const UDP_BUFFER_SIZE: usize = 2048;

/// Подписка клиента на котировки
struct Subscription {
    /// UDP-адрес клиента для отправки котировок
    udp_addr: SocketAddr,
    /// Список тикеров, на которые подписан клиент
    tickers: Vec<Ticker>,
    /// Отправитель для отправки котировок в канал
    sender: Sender<StockQuote>,
    /// Время последнего PING от клиента
    last_ping: Instant,
}

impl Subscription {
    /// Проверяет, подписан ли клиент на данный тикер
    fn is_interested(&self, ticker: &str) -> bool {
        self.tickers.iter().any(|t| t.0 == ticker)
    }
}

/// Состояние сервера
struct ServerState {
    /// Список активных подписок
    subscriptions: Vec<Subscription>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
        }
    }

    /// Добавляет новую подписку
    fn add_subscription(&mut self, subscription: Subscription) {
        self.subscriptions.push(subscription);
    }

    /// Обновляет время последнего PING для подписки
    fn update_ping(&mut self, udp_addr: SocketAddr) -> bool {
        for sub in &mut self.subscriptions {
            if sub.udp_addr == udp_addr {
                sub.last_ping = Instant::now();
                return true;
            }
        }
        false
    }

    /// Удаляет подписки, у которых истёк тайм-аут PING
    fn remove_expired(&mut self) {
        let now = Instant::now();
        self.subscriptions.retain(|sub| {
            if now.duration_since(sub.last_ping) > PING_TIMEOUT {
                println!("Removing expired subscription: {}", sub.udp_addr);
                false
            } else {
                true
            }
        });
    }

    /// Отправляет котировку всем подписчикам, которые интересуются данным тикером
    fn send_quote_to_subscribers(&self, quote: &StockQuote) -> usize {
        let mut sent_count = 0;

        for sub in &self.subscriptions {
            if sub.is_interested(&quote.ticker.0) && sub.sender.send(quote.clone()).is_ok() {
                sent_count += 1;
            }
        }

        sent_count
    }
}

/// Генерирует случайную котировку для тикера
fn generate_quote(ticker: &str, rng: &mut impl RngExt) -> StockQuote {
    let base_price = match ticker {
        "AAPL" => 150.0,
        "MSFT" => 320.0,
        "GOOGL" => 140.0,
        "TSLA" => 250.0,
        "AMZN" => 180.0,
        "NVDA" => 450.0,
        "META" => 330.0,
        "BRK.B" => 410.0,
        "JPM" => 160.0,
        _ => 100.0,
    };

    let price_change: f64 = rng.random_range(-5.0..5.0);
    let price = base_price * (1.0 + price_change / 100.0);
    let price = (price * 100.0).round() / 100.0;

    let volume: u32 = if ["AAPL", "MSFT", "GOOGL", "TSLA", "AMZN", "NVDA", "META"].contains(&ticker)
    {
        rng.random_range(1000..6000)
    } else {
        rng.random_range(100..1100)
    };

    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    StockQuote::new(
        Ticker(ticker.to_string()),
        Price(price),
        Volume(volume),
        Timestamp(timestamp_ms),
    )
}

/// Запускает генератор котировок в отдельном потоке
fn start_generator(tickers: Vec<Ticker>, state: Arc<Mutex<ServerState>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut rng = rng();

        println!("Generator started with {} tickers", tickers.len());

        loop {
            for ticker in &tickers {
                let quote = generate_quote(&ticker.0, &mut rng);

                let state_guard = state.lock().unwrap();
                let _sent_count = state_guard.send_quote_to_subscribers(&quote);
            }

            {
                let mut state_guard = state.lock().unwrap();
                state_guard.remove_expired();
            }

            thread::sleep(GENERATION_INTERVAL);
        }
    })
}

/// Запускает UDP-поток для отправки котировок клиенту
fn start_udp_stream(
    receiver: Receiver<StockQuote>,
    udp_addr: SocketAddr,
    udp_socket: Arc<UdpSocket>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("UDP stream started for {}", udp_addr);

        while let Ok(quote) = receiver.recv() {
            let data = format!("{}\n", quote.to_wire_line());

            match udp_socket.send_to(data.as_bytes(), udp_addr) {
                Ok(_) => (),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::AddrNotAvailable {
                        eprintln!("Client {} is not available, stopping stream", udp_addr);
                        break;
                    }

                    eprintln!("Failed to send quote to {}: {}", udp_addr, e);
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }

        println!("UDP stream stopped for {}", udp_addr);
    })
}

/// Запускает поток для приёма PING-пакетов
fn start_ping_receiver(
    state: Arc<Mutex<ServerState>>,
    udp_socket: Arc<UdpSocket>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0u8; UDP_BUFFER_SIZE];
        println!("PING receiver started");

        loop {
            match udp_socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    let Ok(data) = String::from_utf8(buf[..size].to_vec()) else {
                        continue;
                    };

                    if data.trim() != "PING" {
                        continue;
                    }

                    let mut state_guard = state.lock().unwrap();
                    if !state_guard.update_ping(src_addr) {
                        eprintln!("PING from unknown client: {}", src_addr);
                    }
                }
                Err(e) => {
                    eprintln!("PING receiver error: {}", e);
                }
            }
        }
    })
}

/// Обрабатывает одного клиента в отдельном потоке
fn handle_client(
    mut stream: TcpStream,
    available_tickers: Arc<HashSet<String>>,
    state: Arc<Mutex<ServerState>>,
    udp_socket: Arc<UdpSocket>,
) {
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
            let (sender, receiver) = mpsc::channel();

            let subscription = Subscription {
                udp_addr: command.udp_addr,
                tickers: command.tickers.clone(),
                sender,
                last_ping: Instant::now(),
            };

            {
                let mut state_guard = state.lock().unwrap();
                state_guard.add_subscription(subscription);
                println!("Client {}: Added subscription", client_addr);
            }

            if let Err(e) = stream.write_all(b"OK\n") {
                eprintln!("Client {}: Failed to send OK: {}", client_addr, e);
                return;
            }

            let udp_ref = Arc::clone(&udp_socket);
            let _udp_handle = start_udp_stream(receiver, command.udp_addr, udp_ref);

            println!(
                "Client {}: Subscription OK - {} tickers -> {}",
                client_addr,
                command.tickers.len(),
                command.udp_addr
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
}

fn main() -> std::io::Result<()> {
    let tickers = match tickers::read_tickers_from_file(TICKERS_FILE_PATH) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to read tickers file '{}': {}", TICKERS_FILE_PATH, e);
            std::process::exit(1);
        }
    };

    println!(
        "Loaded {} tickers from {}",
        tickers.len(),
        TICKERS_FILE_PATH
    );

    let tickers_set: HashSet<String> = tickers.iter().map(|ticker| ticker.0.clone()).collect();
    let tickers_set = Arc::new(tickers_set);

    let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
    let udp_socket = Arc::new(udp_socket);
    let udp_port = udp_socket.local_addr()?.port();
    println!("UDP server started on port {}", udp_port);

    let state = Arc::new(Mutex::new(ServerState::new()));
    let generator_handle = start_generator(tickers, Arc::clone(&state));
    let ping_handle = start_ping_receiver(Arc::clone(&state), Arc::clone(&udp_socket));
    let listener = TcpListener::bind(("127.0.0.1", SERVER_PORT))?;

    println!("TCP server listening on 127.0.0.1:{}", SERVER_PORT);
    println!("Press Ctrl+C to stop");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tickers_ref = Arc::clone(&tickers_set);
                let state_ref = Arc::clone(&state);
                let udp_ref = Arc::clone(&udp_socket);
                thread::spawn(move || {
                    handle_client(stream, tickers_ref, state_ref, udp_ref);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    generator_handle.join().unwrap_or_else(|e| {
        eprintln!("Generator thread panicked: {:?}", e);
    });
    ping_handle.join().unwrap_or_else(|e| {
        eprintln!("PING thread panicked: {:?}", e);
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_subscription_is_interested() {
        let tickers = vec![Ticker("AAPL".to_string()), Ticker("MSFT".to_string())];
        let (sender, _receiver) = mpsc::channel();
        let subscription = Subscription {
            udp_addr: "127.0.0.1:9000".parse().unwrap(),
            tickers,
            sender,
            last_ping: Instant::now(),
        };

        assert!(subscription.is_interested("AAPL"));
        assert!(subscription.is_interested("MSFT"));
        assert!(!subscription.is_interested("GOOGL"));
    }

    #[test]
    fn test_server_state_add_subscription() {
        let mut state = ServerState::new();
        let (sender, _receiver) = mpsc::channel();
        let subscription = Subscription {
            udp_addr: "127.0.0.1:9000".parse().unwrap(),
            tickers: vec![Ticker("AAPL".to_string())],
            sender,
            last_ping: Instant::now(),
        };

        state.add_subscription(subscription);
        assert_eq!(state.subscriptions.len(), 1);
    }

    #[test]
    fn test_server_state_update_ping() {
        let mut state = ServerState::new();
        let (sender, _receiver) = mpsc::channel();
        let udp_addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let subscription = Subscription {
            udp_addr,
            tickers: vec![Ticker("AAPL".to_string())],
            sender,
            last_ping: Instant::now(),
        };

        state.add_subscription(subscription);

        assert!(state.update_ping(udp_addr));

        let unknown_addr: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        assert!(!state.update_ping(unknown_addr));
    }

    #[test]
    fn test_server_state_remove_expired() {
        let mut state = ServerState::new();
        let (sender, _receiver) = mpsc::channel();

        let expired_sub = Subscription {
            udp_addr: "127.0.0.1:9000".parse().unwrap(),
            tickers: vec![Ticker("AAPL".to_string())],
            sender: sender.clone(),
            last_ping: Instant::now() - Duration::from_secs(10),
        };
        state.add_subscription(expired_sub);

        let (sender2, _receiver2) = mpsc::channel();
        let active_sub = Subscription {
            udp_addr: "127.0.0.1:9001".parse().unwrap(),
            tickers: vec![Ticker("MSFT".to_string())],
            sender: sender2,
            last_ping: Instant::now(),
        };
        state.add_subscription(active_sub);

        assert_eq!(state.subscriptions.len(), 2);

        state.remove_expired();

        assert_eq!(state.subscriptions.len(), 1);
        assert_eq!(
            state.subscriptions[0].udp_addr,
            "127.0.0.1:9001".parse::<SocketAddr>().unwrap()
        );
    }

    #[test]
    fn test_server_state_send_quote_to_subscribers() {
        let mut state = ServerState::new();

        let (sender1, receiver1) = mpsc::channel();
        let sub1 = Subscription {
            udp_addr: "127.0.0.1:9000".parse().unwrap(),
            tickers: vec![Ticker("AAPL".to_string())],
            sender: sender1,
            last_ping: Instant::now(),
        };
        state.add_subscription(sub1);

        let (sender2, receiver2) = mpsc::channel();
        let sub2 = Subscription {
            udp_addr: "127.0.0.1:9001".parse().unwrap(),
            tickers: vec![Ticker("MSFT".to_string())],
            sender: sender2,
            last_ping: Instant::now(),
        };
        state.add_subscription(sub2);

        let quote = StockQuote::new(
            Ticker("AAPL".to_string()),
            Price(150.0),
            Volume(1000),
            Timestamp(1234567890),
        );

        let sent_count = state.send_quote_to_subscribers(&quote);
        assert_eq!(sent_count, 1);

        let received = receiver1.recv_timeout(Duration::from_millis(100));
        assert!(received.is_ok());

        let not_received = receiver2.recv_timeout(Duration::from_millis(100));
        assert!(not_received.is_err());
    }
}
