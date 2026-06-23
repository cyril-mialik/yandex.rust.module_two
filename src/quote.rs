/// Индекс поля с тикером в строке котировки (0-based)
const TICKER_INDEX: usize = 0;

/// Индекс поля с ценой в строке котировки (0-based)
const PRICE_INDEX: usize = 1;

/// Индекс поля с объёмом в строке котировки (0-based)
const VOLUME_INDEX: usize = 2;

/// Индекс поля с временной меткой в строке котировки (0-based)
const TIMESTAMP_INDEX: usize = 3;

/// Разделитель полей в строке котировки согласно протоколу
const TICKER_SEPARATOR: char = '|';

/// Ожидаемое количество полей в строке котировки:
/// ticker|price|volume|timestamp_ms
const TIKER_LEN: usize = 4;

/// Ошибки, возникающие при парсинге котировки из строки.
#[derive(Debug, PartialEq)]
pub enum ParseQuoteError {
    /// Неверное количество полей (должно быть 4)
    InvalidFieldCount { expected: usize, actual: usize },
    /// Пустой тикер
    EmptyTicker,
    /// Некорректный формат цены
    InvalidPrice { value: String },
    /// Некорректный формат объёма
    InvalidVolume { value: String },
    /// Некорректный формат временной метки
    InvalidTimestamp { value: String },
}

impl std::fmt::Display for ParseQuoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseQuoteError::InvalidFieldCount { expected, actual } => {
                write!(
                    f,
                    "Expected {} fields separated by '{TICKER_SEPARATOR}', got {}",
                    expected, actual
                )
            }
            ParseQuoteError::InvalidPrice { value } => {
                write!(f, "Invalid price format: '{}' (expected number)", value)
            }
            ParseQuoteError::InvalidVolume { value } => {
                write!(
                    f,
                    "Invalid volume format: '{}' (expected unsigned integer)",
                    value
                )
            }
            ParseQuoteError::InvalidTimestamp { value } => {
                write!(
                    f,
                    "Invalid timestamp format: '{}' (expected unsigned integer)",
                    value
                )
            }
            ParseQuoteError::EmptyTicker => {
                write!(f, "The ticker cannot be empty")
            }
        }
    }
}

impl std::error::Error for ParseQuoteError {}

/// Структура, представляющая тикер (символ) актика, например "AAPL" или "BRK.B".
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker(pub String);

/// Структура, представляющая цену актива с плавающей точкой.
#[derive(Debug, Clone, PartialEq)]
pub struct Price(pub f64);

/// Структура, представляющая обьем торгов в штуках (целое число)
#[derive(Debug, Clone, PartialEq)]
pub struct Volume(pub u32);

/// Структура, представляющее временную метку в миллисекундах (unix time)
#[derive(Debug, Clone, PartialEq)]
pub struct Timestamp(pub u64);

impl std::fmt::Display for Ticker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Volume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Структура, представляющая биржевую котировку.
///
/// Используется для передачи данных между сервером и клиентом
/// через UDP в текстовом формате.
#[derive(Debug, Clone, PartialEq)]
pub struct StockQuote {
    /// Тикер (символ) актива, например "AAPL" или "BRK.B"
    pub ticker: Ticker,

    /// Цена актива с плавающей точкой
    pub price: Price,

    /// Объём торгов в штуках (целое число)
    pub volume: Volume,

    /// Временная метка в миллисекундах (Unix time)
    pub timestamp_ms: Timestamp,
}

impl StockQuote {
    /// Создаёт новую котировку
    pub fn new(ticker: Ticker, price: Price, volume: Volume, timestamp_ms: Timestamp) -> Self {
        Self {
            ticker,
            price,
            volume,
            timestamp_ms,
        }
    }

    /// Преобразует котировку в строку для передачи по сети.
    ///
    /// Формат: `ticker|price|volume|timestamp_ms`
    ///
    /// # Пример
    /// ```
    /// use homework_module_two_streaming_quotes::quote::{StockQuote, Ticker, Price, Volume, Timestamp};
    ///
    /// let quote = StockQuote::new(
    ///     Ticker("AAPL".to_string()),
    ///     Price(150.25),
    ///     Volume(4200),
    ///     Timestamp(1710000000123),
    /// );
    ///
    /// assert_eq!(
    ///     quote.to_wire_line(),
    ///     "AAPL|150.25|4200|1710000000123"
    /// );
    /// ```
    pub fn to_wire_line(&self) -> String {
        format!(
            "{}{TICKER_SEPARATOR}{}{TICKER_SEPARATOR}{}{TICKER_SEPARATOR}{}",
            self.ticker, self.price, self.volume, self.timestamp_ms
        )
    }

    /// Парсит строку в котировку.
    ///
    /// # Формат
    /// Ожидается строка из 4 полей, разделённых символом `|`:
    /// `ticker|price|volume|timestamp_ms`
    ///
    /// # Ошибки
    /// Возвращает `ParseQuoteError` в случае:
    /// - Неверного количества полей
    /// - Некорректного формата цены, объёма или временной метки
    ///
    /// # Пример
    /// ```
    /// use homework_module_two_streaming_quotes::quote::{StockQuote, ParseQuoteError};
    ///
    /// let line = "AAPL,150.00,100,1625097600";
    /// # Ok::<_, ParseQuoteError>(())
    /// ```
    pub fn from_wire_line(line: &str) -> Result<Self, ParseQuoteError> {
        let parts: Vec<&str> = line.split(TICKER_SEPARATOR).collect();

        if parts.len() != TIKER_LEN {
            return Err(ParseQuoteError::InvalidFieldCount {
                expected: TIKER_LEN,
                actual: parts.len(),
            });
        }

        let ticker_raw = parts[TICKER_INDEX].trim().to_string();
        if ticker_raw.is_empty() {
            return Err(ParseQuoteError::EmptyTicker);
        }

        let ticker = Ticker(ticker_raw);

        let price = Price(parts[PRICE_INDEX].parse::<f64>().map_err(|_| {
            ParseQuoteError::InvalidPrice {
                value: parts[PRICE_INDEX].to_string(),
            }
        })?);

        let volume = Volume(parts[VOLUME_INDEX].parse::<u32>().map_err(|_| {
            ParseQuoteError::InvalidVolume {
                value: parts[VOLUME_INDEX].to_string(),
            }
        })?);

        let timestamp_ms = Timestamp(parts[TIMESTAMP_INDEX].parse::<u64>().map_err(|_| {
            ParseQuoteError::InvalidTimestamp {
                value: parts[TIMESTAMP_INDEX].to_string(),
            }
        })?);

        Ok(StockQuote {
            ticker,
            price,
            volume,
            timestamp_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_wire_line_ok() {
        let line = "AAPL|150.25|4200|1710000000123";
        let quote = StockQuote::from_wire_line(line).unwrap();

        assert_eq!(quote.ticker.0, "AAPL");
        assert_eq!(quote.price.0, 150.25);
        assert_eq!(quote.volume.0, 4200);
        assert_eq!(quote.timestamp_ms.0, 1710000000123);
    }

    #[test]
    fn test_to_wire_line_format() {
        let quote = StockQuote::new(
            Ticker("AAPL".to_string()),
            Price(150.25),
            Volume(4200),
            Timestamp(1710000000123),
        );

        assert_eq!(quote.to_wire_line(), "AAPL|150.25|4200|1710000000123");
    }

    #[test]
    fn test_roundtrip() {
        let original = StockQuote::new(
            Ticker("BRK.B".to_string()),
            Price(123.456),
            Volume(100),
            Timestamp(1234567890),
        );

        let wire = original.to_wire_line();
        let parsed = StockQuote::from_wire_line(&wire).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_from_wire_line_errors() {
        let test_cases = vec![
            (
                "AAPL|150.25|4200",
                ParseQuoteError::InvalidFieldCount {
                    expected: 4,
                    actual: 3,
                },
            ),
            (
                "AAPL|invalid|4200|123",
                ParseQuoteError::InvalidPrice {
                    value: "invalid".to_string(),
                },
            ),
            (
                "AAPL|150.25|invalid|123",
                ParseQuoteError::InvalidVolume {
                    value: "invalid".to_string(),
                },
            ),
            (
                "AAPL|150.25|4200|invalid",
                ParseQuoteError::InvalidTimestamp {
                    value: "invalid".to_string(),
                },
            ),
            ("|150.25|4200|123", ParseQuoteError::EmptyTicker),
            (" |150.25|4200|123", ParseQuoteError::EmptyTicker),
        ];

        for (input, expected_error) in test_cases {
            let result = StockQuote::from_wire_line(input);
            assert_eq!(result, Err(expected_error));
        }
    }

    #[test]
    fn test_ticker_trim() {
        let line = " AAPL |150.25|4200|123";
        let quote = StockQuote::from_wire_line(line).unwrap();
        assert_eq!(quote.ticker.0, "AAPL");
    }

    #[test]
    fn test_display_impls() {
        let ticker = Ticker("AAPL".to_string());
        let price = Price(150.25);
        let volume = Volume(4200);
        let timestamp = Timestamp(1710000000123);

        assert_eq!(format!("{}", ticker), "AAPL");
        assert_eq!(format!("{}", price), "150.25");
        assert_eq!(format!("{}", volume), "4200");
        assert_eq!(format!("{}", timestamp), "1710000000123");
    }
}
