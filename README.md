# Doggy Watch

Telegram бот для предложения YouTube видео на стрим.
Сделан специально для [Doggy Dox](https://www.twitch.tv/doggy_dox).

## Переменные

`DATABASE_URL=postgres://<username>:<password>@<address>/<database>`

Параметры базы данных

`TOKEN=<bot_token>`

Токен бота. Можно получить у [@botfather](tg://resolve?domain=botfather)

`ADMINISTRATORS=<user_id>[,<user_id>,...]`

ID администраторов, разделяется запятой.
Можно получить у [@getmyid_bot](tg://resolve?domain=getmyid_bot)

`CHANNEL=<chat_id>`

ID канала для проверки подписки.
Можно получить у [@getmyid_bot](tg://resolve?domain=getmyid_bot) переслав ему сообщения из канала.

`CHANNEL_INVITE_HASH=<hash>`

Хэш для инвайт ссылки (необязательно). Хэш можно извлечь из ссылки-приглашения после плюса.
Пример: `https://t.me/+<hash>`

`RUST_LOG=<level>[,target=level,...]`

Журналирование (необязательно).
Типы:
`trace, debug, info, warn, error`
Также можно указать отдельный уровень логирования для отдельных целей.

### Только для Docker

`TZ=<TZ_identifier>`

Необязательно, но рекомендуется, т.к. данные в БД хранятся без часового пояса.
Можно взять из [таблицы с Википедии](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones)

## Contributing

![Спроси меня о чём угодно!](https://img.shields.io/badge/Ask%20me-anything-1abc9c.svg)
в
[![Telegram](https://badgen.net/static/icon/Telegram?icon=telegram&color=cyan&label)](https://t.me/shiroyashik)
или
![Discord](https://badgen.net/badge/icon/Discord?icon=discord&label)

Если у вас есть идеи, нашли баг или хотите предложить улучшения:
создавайте [issue](https://github.com/shiroyashik/doggy-watch/issues)
или свяжитесь со мной напрямую через Discord/Telegram (**@shiroyashik**).

Если вы Rust разработчик, буду рад вашим Pull Request'ам:

1. Форкните репу
2. Создайте новую ветку
3. Создайте PR!

Буду рад любой вашей помощи! ❤

---

**АХТУНГ!** В исходниках матюки! **:3**

## License

Doggy Watch is licensed under [GPL-3.0](LICENSE)
