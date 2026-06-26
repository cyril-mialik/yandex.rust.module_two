# Streaming Quotes
### Биржевой стриминг-сервис на Rust с TCP-управлением и UDP-передачей данных.

__Быстрый старт__:

```bash
# Запуск сервера
cargo run --bin server

# Запуск клиента (порт 9000 — UDP-порт для приёма котировок)
cargo run --bin client -- 127.0.0.1:7878 9000 assets/tickers.txt
```

__Сборка__:
```bash
cargo build
```

__Тикеры__:
| Тикер | Компания |
| -------- | -------- | 
|  AAPL | Apple Inc. |
|  MSFT | Microsoft Corporation | 
|  GOOGL | Alphabet Inc. (Class A) | 
|  TSLA	| Tesla Inc. | 
|  AMZN	| Amazon.com Inc. | 
|  NVDA	| NVIDIA Corporation | 
|  META	| Meta Platforms Inc. | 
|  BRK.B | Berkshire Hathaway Inc. | 
|  JPM | JPMorgan Chase & Co. | 

__Архитектура__:
- __TCP__ — управление: подписка _STREAM_, ответы OK/ERR
- __UDP__ — передача котировок и _PING_ для keep-alive
- __Многопоточность__: генератор, обработчики клиентов, UDP-потоки

__Протокол__

Подписка (TCP)
```text
STREAM <udp_addr> <ticker1,ticker2,...>
```
Ответы:
- `OK\n` — успешная подписка
- `ERR invalid command\n` — неверный формат
- `ERR invalid udp address\n` — неверный адрес
- `ERR empty ticker list\n` — пустой список
- `ERR unknown ticker: <TICKER>\n` — неизвестный тикер

Котировка (UDP)
```text
ticker|price|volume|timestamp_ms
```
- Поля разделяются |
- Одна котировка на датаграмму
- Буфер приёма — 2 KiB
- Пробелы внутри полей не допускаются

Keep-Alive
- Клиент шлёт PING на UDP-адрес сервера каждые 2 секунды
- Тайм-аут без PING — 5 секунд
- При тайм-ауте сервер закрывает UDP-стрим

__Структура проекта__:

```text
src/
├── bin/
│   ├── server.rs          # TCP-сервер, генератор, UDP-отправка
│   └── client.rs          # Подписка, приём котировок, PING
├── lib.rs
├── quote.rs               # StockQuote, парсинг/сериализация
├── protocol.rs            # Парсинг STREAM, коды ответов
└── tickers.rs             # Чтение файла тикеров
```

__Проверка фильтрации__:
1. Запустите сервер:
```bash
cargo run --bin server
```
2. Запустите клиент с подмножеством тикеров:
```bash
cargo run --bin client -- 127.0.0.1:7878 9000 assets/tickers.txt
```
3. Проверьте, что в консоль выводятся только запрошенные тикеры
4. Запустите второй клиент с другим набором тикеров:
```bash
cargo run --bin client -- 127.0.0.1:7878 9001 assets/tickers.txt
```
5. Убедитесь, что оба клиента получают только свои тикеры

__Проверка тайм-аута PING__:
1. Запустите сервер и клиент
2. Остановите клиент (Ctrl+C)
3. На сервере через ~5 секунд появится сообщение:
```text
Removing expired subscription: 127.0.0.1:9000
```
4. UDP-стрим для этого клиента будет закрыт

__Тестирование через netcat__:
```bash
# Успешная подписка
echo "STREAM 127.0.0.1:9000 AAPL,MSFT" | nc 127.0.0.1 7878
# OK

# Неверная команда
echo "STREEM 127.0.0.1:9000 AAPL" | nc 127.0.0.1 7878
# ERR invalid command

# Неверный адрес
echo "STREAM not-an-address AAPL" | nc 127.0.0.1 7878
# ERR invalid udp address

# Неизвестный тикер
echo "STREAM 127.0.0.1:9000 UNKNOWN" | nc 127.0.0.1 7878
# ERR unknown ticker: UNKNOWN

# Пустой список тикеров
echo "STREAM 127.0.0.1:9000 " | nc 127.0.0.1 7878
# ERR empty ticker list
```

__Конфигурация__:
Порты и интервалы задаются константами в server.rs и client.rs:

| Константа	| Значение | Где |
| ---- | ---- | ---- |
| SERVER_PORT | 7878 | server.rs |
| GENERATION_INTERVAL | 100ms | server.rs |
| PING_INTERVAL | 2s | client.rs |
| PING_TIMEOUT | 5s | server.rs |

__Известные ограничения__:
1. Только IPv4 — поддержка IPv6 не реализована
2. Только localhost — сервер привязан к 127.0.0.1
3. Одна команда на TCP-соединение — несколько команд подряд не поддерживаются
4. Нет graceful shutdown — завершение через Ctrl+C
5. Нет аргументов командной строки — порты и пути задаются константами
