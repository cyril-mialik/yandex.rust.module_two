use crate::quote::Ticker;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// Ошибки при чтении файла тикеров
#[derive(Debug, PartialEq)]
pub enum TickersError {
    /// Файл не найден
    FileNotFound,
    /// Файл пуст или содержит только пустые строки
    EmptyFile,
    /// Ошибка чтения файла (IO)
    IoError(String),
}

impl std::fmt::Display for TickersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TickersError::FileNotFound => write!(f, "Tickers file not found"),
            TickersError::EmptyFile => write!(f, "Tickers file is empty"),
            TickersError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for TickersError {}

impl From<std::io::Error> for TickersError {
    fn from(error: std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            TickersError::FileNotFound
        } else {
            TickersError::IoError(error.to_string())
        }
    }
}

/// Читает тикеры из любого источника, реализующего `Read`
fn read_tickers_from_reader<R: Read>(reader: R) -> Result<Vec<Ticker>, TickersError> {
    let reader = BufReader::new(reader);
    let mut tickers = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Пропускаем пустые строки
        if trimmed.is_empty() {
            continue;
        }

        tickers.push(Ticker(trimmed.to_string()));
    }

    // Проверяем, что файл не пустой
    if tickers.is_empty() {
        return Err(TickersError::EmptyFile);
    }

    Ok(tickers)
}

/// Читает файл с тикерами и возвращает вектор тикеров.
///
/// # Формат файла
/// - Один тикер на строку
/// - Пустые строки пропускаются
/// - Пробелы по краям обрезаются (`trim()`)
///
/// # Аргументы
/// * `path` - путь к файлу с тикерами
///
/// # Возвращаемое значение
/// * `Ok(Vec<Ticker>)` - список тикеров
/// * `Err(TickersError)` - ошибка чтения или пустой файл
///
/// # Пример
/// ```
/// use homework_module_two_streaming_quotes::tickers::read_tickers_from_file;
///
/// let tickers = read_tickers_from_file("assets/tickers.txt").unwrap();
/// assert!(!tickers.is_empty());
/// ```
pub fn read_tickers_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Ticker>, TickersError> {
    let file = File::open(path)?;
    read_tickers_from_reader(file)
}

/// Читает файл с тикерами и возвращает HashSet для быстрого поиска.
///
/// Это удобно для валидации тикеров в команде STREAM.
///
/// # Пример
/// ```
/// use std::collections::HashSet;
/// use homework_module_two_streaming_quotes::tickers::read_tickers_to_set;
///
/// let tickers_set = read_tickers_to_set("assets/tickers.txt").unwrap();
/// assert!(tickers_set.contains("AAPL"));
/// ```
pub fn read_tickers_to_set<P: AsRef<Path>>(path: P) -> Result<HashSet<String>, TickersError> {
    let tickers = read_tickers_from_file(path)?;
    let mut set = HashSet::new();

    for ticker in tickers {
        set.insert(ticker.0);
    }

    Ok(set)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_tickers_from_file_ok() {
        let data = "AAPL\nMSFT\nTSLA\n";
        let cursor = Cursor::new(data);
        let result = read_tickers_from_reader(cursor).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "AAPL");
        assert_eq!(result[1].0, "MSFT");
        assert_eq!(result[2].0, "TSLA");
    }

    #[test]
    fn test_read_tickers_with_empty_lines() {
        let data = "AAPL\n\nMSFT\n\n\nTSLA\n";
        let cursor = Cursor::new(data);
        let result = read_tickers_from_reader(cursor).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "AAPL");
        assert_eq!(result[1].0, "MSFT");
        assert_eq!(result[2].0, "TSLA");
    }

    #[test]
    fn test_read_tickers_with_spaces() {
        let data = "  AAPL  \n  MSFT  \n  TSLA  \n";
        let cursor = Cursor::new(data);
        let result = read_tickers_from_reader(cursor).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "AAPL");
        assert_eq!(result[1].0, "MSFT");
        assert_eq!(result[2].0, "TSLA");
    }

    #[test]
    fn test_read_tickers_with_ticker_containing_dot() {
        let data = "AAPL\nBRK.B\nMSFT\n";
        let cursor = Cursor::new(data);
        let result = read_tickers_from_reader(cursor).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "AAPL");
        assert_eq!(result[1].0, "BRK.B");
        assert_eq!(result[2].0, "MSFT");
    }

    #[test]
    fn test_read_tickers_empty_file() {
        let data = "";
        let cursor = Cursor::new(data);
        let result = read_tickers_from_reader(cursor);

        assert_eq!(result, Err(TickersError::EmptyFile));
    }

    #[test]
    fn test_read_tickers_only_empty_lines() {
        let data = "\n\n\n  \n  \n";
        let cursor = Cursor::new(data);
        let result = read_tickers_from_reader(cursor);

        assert_eq!(result, Err(TickersError::EmptyFile));
    }

    #[test]
    fn test_read_tickers_file_not_found() {
        let result = read_tickers_from_file("non_existent_file_123456789.txt");

        assert_eq!(result, Err(TickersError::FileNotFound));
    }

    #[test]
    fn test_read_tickers_to_set() {
        let data = "AAPL\nMSFT\nTSLA\nAAPL\n";
        let cursor = Cursor::new(data);
        let tickers = read_tickers_from_reader(cursor).unwrap();

        let mut set = HashSet::new();
        for ticker in tickers {
            set.insert(ticker.0);
        }

        assert_eq!(set.len(), 3);
        assert!(set.contains("AAPL"));
        assert!(set.contains("MSFT"));
        assert!(set.contains("TSLA"));
    }

    #[test]
    fn test_tickers_error_display() {
        let error = TickersError::FileNotFound;
        assert_eq!(error.to_string(), "Tickers file not found");

        let error = TickersError::EmptyFile;
        assert_eq!(error.to_string(), "Tickers file is empty");

        let error = TickersError::IoError("permission denied".to_string());
        assert_eq!(error.to_string(), "IO error: permission denied");
    }
}
