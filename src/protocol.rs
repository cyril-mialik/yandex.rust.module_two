use crate::quote::Ticker;
use std::collections::HashSet;
use std::net::SocketAddr;

/// Префикс команды STREAM
const STREAM_PREFIX: &str = "STREAM";

/// Индекс части команды, содержащей UDP-адрес (после `splitn`)
/// Формат: `STREAM <addr> <tickers>`
/// Индексы:     0       1       2
const COMMAND_ADDR_INDEX: usize = 1;

/// Индекс части команды, содержащей список тикеров (после `splitn`)
/// Формат: `STREAM <addr> <tickers>`
/// Индексы:     0       1       2
const COMMAND_TICKERS_INDEX: usize = 2;

/// Ожидаемое количество частей команды после разбиения `splitn(3, ' ')`
/// Части: ["STREAM", "<addr>", "<tickers>"]
const COMMAND_LEN: usize = 3;

/// Разделитель между тикерами в списке (запятая)
/// Формат: `ticker1,ticker2,ticker3`
const TICKER_SEPARATOR: char = ',';

/// Разделитель между командами (пробел)
/// Формат: `STREAM <udp_addr> <ticker1,ticker2,...>`
const COMMAND_SEPARATOR: char = ' ';

/// Ошибки, возникающие при парсинге команды STREAM
#[derive(Debug, PartialEq)]
pub enum ParseStreamError {
    /// Неверный формат команды (не начинается с STREAM или неверное количество частей)
    InvalidCommand,
    /// Некорректный UDP-адрес
    InvalidUdpAddress,
    /// Пустой список тикеров
    EmptyTickerList,
    /// Неизвестный тикер (не найден в справочнике)
    UnknownTicker { ticker: String },
}

impl std::fmt::Display for ParseStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseStreamError::InvalidCommand => {
                write!(f, "Invalid command format")
            }
            ParseStreamError::InvalidUdpAddress => {
                write!(f, "Invalid UDP address format")
            }
            ParseStreamError::EmptyTickerList => {
                write!(f, "Ticker list is empty")
            }
            ParseStreamError::UnknownTicker { ticker } => {
                write!(f, "Unknown ticker: {}", ticker)
            }
        }
    }
}

impl std::error::Error for ParseStreamError {}

/// Результат успешного парсинга команды STREAM
#[derive(Debug, Clone, PartialEq)]
pub struct StreamCommand {
    /// UDP-адрес, на который клиент ожидает котировки
    pub udp_addr: SocketAddr,
    /// Список тикеров, на которые подписывается клиент
    pub tickers: Vec<Ticker>,
}

/// Парсит команду STREAM из строки и валидирует тикеры по справочнику.
///
/// # Формат
/// `STREAM <udp_addr> <ticker1,ticker2,...>`
///
/// # Аргументы
/// * `line` - строка с командой (должна заканчиваться на `\n`)
/// * `available_tickers` - множество доступных тикеров (из файла)
///
/// # Возвращаемое значение
/// * `Ok(StreamCommand)` - успешно распарсенный результат
/// * `Err(ParseStreamError)` - ошибка валидации
///
/// # Пример
/// ```
/// use std::collections::HashSet;
/// use homework_module_two_streaming_quotes::protocol::parse_stream_command;
/// use homework_module_two_streaming_quotes::quote::Ticker;
///
/// let mut available = HashSet::new();
/// available.insert("AAPL".to_string());
/// available.insert("MSFT".to_string());
///
/// let result = parse_stream_command(
///     "STREAM 127.0.0.1:9000 AAPL,MSFT",
///     &available,
/// ).unwrap();
///
/// assert_eq!(result.udp_addr.to_string(), "127.0.0.1:9000");
/// assert_eq!(result.tickers, vec![Ticker("AAPL".to_string()), Ticker("MSFT".to_string())]);
/// ```
pub fn parse_stream_command(
    line: &str,
    available_tickers: &HashSet<String>,
) -> Result<StreamCommand, ParseStreamError> {
    let trimmed = line.trim_start().trim_end_matches('\n');

    if !trimmed.starts_with(STREAM_PREFIX) {
        return Err(ParseStreamError::InvalidCommand);
    }

    let parts: Vec<&str> = trimmed.splitn(COMMAND_LEN, COMMAND_SEPARATOR).collect();
    if parts.len() != COMMAND_LEN {
        return Err(ParseStreamError::InvalidCommand);
    }

    let addr_str = parts[COMMAND_ADDR_INDEX].trim();
    let tickers_str = parts[COMMAND_TICKERS_INDEX].trim();

    if tickers_str.is_empty() {
        return Err(ParseStreamError::EmptyTickerList);
    }

    let ticker_parts: Vec<&str> = tickers_str.split(',').collect();

    let has_ticker = ticker_parts.iter().any(|s| !s.trim().is_empty());
    if !has_ticker {
        return Err(ParseStreamError::EmptyTickerList);
    }

    for part in ticker_parts {
        let trimmed_part = part.trim();
        if !trimmed_part.is_empty() && trimmed_part.contains(' ') {
            return Err(ParseStreamError::InvalidCommand);
        }
    }

    let udp_addr = addr_str
        .parse::<SocketAddr>()
        .map_err(|_| ParseStreamError::InvalidUdpAddress)?;

    let tickers = parse_ticker_list(tickers_str, available_tickers)?;

    Ok(StreamCommand { udp_addr, tickers })
}

/// Парсит строку со списком тикеров и валидирует каждый по справочнику.
///
/// # Формат
/// `ticker1,ticker2,ticker3` (пробелы вокруг запятых допустимы)
fn parse_ticker_list(
    tickers_str: &str,
    available_tickers: &HashSet<String>,
) -> Result<Vec<Ticker>, ParseStreamError> {
    let raw_tickers: Vec<String> = tickers_str
        .split(TICKER_SEPARATOR)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if raw_tickers.is_empty() {
        return Err(ParseStreamError::EmptyTickerList);
    }

    let mut result = Vec::new();
    for ticker_str in raw_tickers {
        if !available_tickers.contains(&ticker_str) {
            return Err(ParseStreamError::UnknownTicker { ticker: ticker_str });
        }

        result.push(crate::quote::Ticker(ticker_str));
    }

    Ok(result)
}

/// Возвращает ответ OK
pub fn ok_response() -> String {
    "OK\n".to_string()
}

/// Возвращает ответ ERR invalid command
pub fn err_invalid_command() -> String {
    "ERR invalid command\n".to_string()
}

/// Возвращает ответ ERR invalid udp address
pub fn err_invalid_udp_address() -> String {
    "ERR invalid udp address\n".to_string()
}

/// Возвращает ответ ERR empty ticker list
pub fn err_empty_ticker_list() -> String {
    "ERR empty ticker list\n".to_string()
}

/// Возвращает ответ ERR unknown ticker: <TICKER>
pub fn err_unknown_ticker(ticker: &str) -> String {
    format!("ERR unknown ticker: {}\n", ticker)
}

/// Преобразует ошибку парсинга в строку ответа
pub fn error_to_response(error: &ParseStreamError) -> String {
    match error {
        ParseStreamError::InvalidCommand => err_invalid_command(),
        ParseStreamError::InvalidUdpAddress => err_invalid_udp_address(),
        ParseStreamError::EmptyTickerList => err_empty_ticker_list(),
        ParseStreamError::UnknownTicker { ticker } => err_unknown_ticker(ticker),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quote::Ticker;
    use std::collections::HashSet;

    fn create_available_tickers() -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("AAPL".to_string());
        set.insert("MSFT".to_string());
        set.insert("TSLA".to_string());
        set.insert("BRK.B".to_string());
        set
    }

    #[test]
    fn test_parse_valid_command() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 AAPL,MSFT\n", &available).unwrap();

        assert_eq!(result.udp_addr.to_string(), "127.0.0.1:9000");
        assert_eq!(
            result.tickers,
            vec![Ticker("AAPL".to_string()), Ticker("MSFT".to_string())]
        );
    }

    #[test]
    fn test_parse_command_with_spaces() {
        let available = create_available_tickers();
        let result =
            parse_stream_command("STREAM 127.0.0.1:9000  AAPL,  MSFT , TSLA  \n", &available)
                .unwrap();

        assert_eq!(result.udp_addr.to_string(), "127.0.0.1:9000");
        assert_eq!(
            result.tickers,
            vec![
                Ticker("AAPL".to_string()),
                Ticker("MSFT".to_string()),
                Ticker("TSLA".to_string())
            ]
        );
    }

    #[test]
    fn test_parse_command_with_ticker_containing_dot() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 BRK.B\n", &available).unwrap();

        assert_eq!(result.udp_addr.to_string(), "127.0.0.1:9000");
        assert_eq!(result.tickers, vec![Ticker("BRK.B".to_string())]);
    }

    #[test]
    fn test_parse_command_without_newline() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 AAPL,MSFT", &available).unwrap();

        assert_eq!(result.udp_addr.to_string(), "127.0.0.1:9000");
        assert_eq!(result.tickers.len(), 2);
    }

    #[test]
    fn test_error_invalid_command_no_prefix() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREEM 127.0.0.1:9000 AAPL\n", &available);
        assert_eq!(result, Err(ParseStreamError::InvalidCommand));
    }

    #[test]
    fn test_error_invalid_command_wrong_parts() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 AAPL MSFT\n", &available);
        assert_eq!(result, Err(ParseStreamError::InvalidCommand));
    }

    #[test]
    fn test_error_invalid_udp_address() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM not-an-address AAPL\n", &available);
        assert_eq!(result, Err(ParseStreamError::InvalidUdpAddress));
    }

    #[test]
    fn test_error_empty_ticker_list() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 \n", &available);
        assert_eq!(result, Err(ParseStreamError::EmptyTickerList));
    }

    #[test]
    fn test_error_empty_ticker_list_only_commas() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 , , \n", &available);
        assert_eq!(result, Err(ParseStreamError::EmptyTickerList));
    }

    #[test]
    fn test_error_unknown_ticker() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 AAPL,GOOGL\n", &available);
        assert_eq!(
            result,
            Err(ParseStreamError::UnknownTicker {
                ticker: "GOOGL".to_string()
            })
        );
    }

    #[test]
    fn test_error_unknown_ticker_first() {
        let available = create_available_tickers();
        let result = parse_stream_command("STREAM 127.0.0.1:9000 GOOGL,AAPL\n", &available);
        assert_eq!(
            result,
            Err(ParseStreamError::UnknownTicker {
                ticker: "GOOGL".to_string()
            })
        );
    }

    #[test]
    fn test_ok_response() {
        assert_eq!(ok_response(), "OK\n");
    }

    #[test]
    fn test_err_invalid_command() {
        assert_eq!(err_invalid_command(), "ERR invalid command\n");
    }

    #[test]
    fn test_err_invalid_udp_address() {
        assert_eq!(err_invalid_udp_address(), "ERR invalid udp address\n");
    }

    #[test]
    fn test_err_empty_ticker_list() {
        assert_eq!(err_empty_ticker_list(), "ERR empty ticker list\n");
    }

    #[test]
    fn test_err_unknown_ticker() {
        assert_eq!(err_unknown_ticker("AAPL"), "ERR unknown ticker: AAPL\n");
    }

    #[test]
    fn test_error_to_response() {
        let error = ParseStreamError::InvalidCommand;
        assert_eq!(error_to_response(&error), "ERR invalid command\n");

        let error = ParseStreamError::InvalidUdpAddress;
        assert_eq!(error_to_response(&error), "ERR invalid udp address\n");

        let error = ParseStreamError::EmptyTickerList;
        assert_eq!(error_to_response(&error), "ERR empty ticker list\n");

        let error = ParseStreamError::UnknownTicker {
            ticker: "GOOGL".to_string(),
        };
        assert_eq!(error_to_response(&error), "ERR unknown ticker: GOOGL\n");
    }

    #[test]
    fn test_parse_display_roundtrip() {
        let available = create_available_tickers();
        let original = "STREAM 127.0.0.1:9000 AAPL,MSFT,TSLA\n";
        let parsed = parse_stream_command(original, &available).unwrap();

        assert_eq!(parsed.udp_addr.to_string(), "127.0.0.1:9000");
        assert_eq!(parsed.tickers.len(), 3);
        assert_eq!(parsed.tickers[0].0, "AAPL");
        assert_eq!(parsed.tickers[1].0, "MSFT");
        assert_eq!(parsed.tickers[2].0, "TSLA");
    }
}
