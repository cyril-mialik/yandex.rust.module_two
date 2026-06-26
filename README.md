# Streaming Quotes
### Биржевой стриминг-сервис на Rust с TCP-управлением и UDP-передачей данных.

__Быстрый старт__:

```bash
# Запуск сервера
cargo run --bin server

# Запуск клиента (порт 9000 — UDP-порт для приёма котировок)
cargo run --bin client -- 127.0.0.1:7878 9000 assets/tickers.txt
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
Ответы: OK или ERR <reason>

Котировка (UDP)
```text
ticker|price|volume|timestamp_ms
```
Поля разделяются `|`, одна котировка на датаграмму, буфер 2 KiB.

Keep-Alive
- Клиент шлёт PING на UDP-адрес сервера каждые 2 секунды. Тайм-аут — 5 секунд.

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

__Тестирование__:
```bash
# Успешная подписка
echo "STREAM 127.0.0.1:9000 AAPL,MSFT" | nc 127.0.0.1 7878
# -> OK

# Ошибки
echo "STREEM 127.0.0.1:9000 AAPL" | nc 127.0.0.1 7878
# -> ERR invalid command

echo "STREAM not-an-address AAPL" | nc 127.0.0.1 7878
# -> ERR invalid udp address

echo "STREAM 127.0.0.1:9000 UNKNOWN" | nc 127.0.0.1 7878
# -> ERR unknown ticker: UNKNOWN
```

__Конфигурация__:
Порты и интервалы задаются константами в server.rs и client.rs:
- SERVER_PORT = 7878
- GENERATION_INTERVAL = 100ms
- PING_INTERVAL = 2s
- PING_TIMEOUT = 5s
