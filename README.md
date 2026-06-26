### homework

__Описание файла__:
| Тикер | Компания | Особенность |
| -------- | -------- | -------- |
|  AAPL | Apple Inc. | Крупный тикер (объём 1000-6000) |
|  MSFT | Microsoft Corporation | Крупный тикер (объём 1000-6000) |
|  GOOGL | Alphabet Inc. (Class A) | Крупный тикер (объём 1000-6000) |
|  TSLA	| Tesla Inc. | Крупный тикер (объём 1000-6000) |
|  AMZN	| Amazon.com Inc. | Крупный тикер (объём 1000-6000) |
|  NVDA	| NVIDIA Corporation | Крупный тикер (объём 1000-6000) |
|  META	| Meta Platforms Inc. | Крупный тикер (объём 1000-6000) |
|  BRK.B | Berkshire Hathaway Inc. | Содержит точку (проверка парсинга) |
|  JPM | JPMorgan Chase & Co. | Обычный тикер (объём 100-1100) |

__Правила формата__:

- Поля разделяются символом |
- Строка не содержит завершающего \n
- Пробелы внутри полей не допускаются

__Тестирование server.rs__
_необходимо будет поставить пакет netcat*_

```bash
# Успешная подписка
echo "STREAM 127.0.0.1:9000 AAPL,MSFT" | nc 127.0.0.1 7878
# Ожидаем: OK

# Неверная команда
echo "STREEM 127.0.0.1:9000 AAPL" | nc 127.0.0.1 7878
# Ожидаем: ERR invalid command

# Неверный адрес
echo "STREAM not-an-address AAPL" | nc 127.0.0.1 7878
# Ожидаем: ERR invalid udp address

# Неизвестный тикер
echo "STREAM 127.0.0.1:9000 UNKNOWN" | nc 127.0.0.1 7878
# Ожидаем: ERR unknown ticker: UNKNOWN
```

