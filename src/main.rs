use std::{env::var, sync::Arc, time::Duration};

use chrono::Local;
use dashmap::DashMap;
use indexmap::IndexMap;
use sea_orm::{prelude::*, ActiveValue::*, Database, Order, QueryOrder};
use teloxide::{
    dispatching::dialogue::{GetChatId, InMemStorage}, prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile, LinkPreviewOptions, ParseMode, User}, utils::{command::BotCommands, html::user_mention}
};
use tokio::time::Instant;
use tracing_panic::panic_hook;
use lazy_static::lazy_static;

const COOLDOWN_DURATION: Duration = Duration::from_secs(30);
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref LOGGER_ENV: String = {
        var("RUST_LOG").unwrap_or(String::from("info"))
    };
    pub static ref TOKEN: String = {
        var("TOKEN").expect("TOKEN env not set.")
    };
    pub static ref DATABASE_URL: String = {
        var("DATABASE_URL").expect("DATABASE_URL env not set.")
    };
    pub static ref ADMINISTRATORS: Vec<u64> = {
        var("ADMINISTRATORS").unwrap_or(String::from(""))
        .split(',').filter_map(|s| s.parse().ok()).collect()
    };
    pub static ref CHANNEL: i64 = {
        var("CHANNEL").expect("TOKEN env not set.").parse().expect("Cant't parse env CHANNEL to i64.")
    };
}

struct AppState {
    db: DatabaseConnection,
    administrators: Vec<u64>,
    cooldown: DashMap<u64, Instant>
    
}

impl AppState {
    async fn check_rights(&self, uid: &UserId) -> anyhow::Result<Rights> {
        use entity::moderators::Entity as Moderators;
        
        Ok(if self.administrators.contains(&uid.0) {
            Rights::Administrator
        } else if Moderators::find_by_id(uid.to_string()).one(&self.db).await?.is_some() {
            Rights::Moderator
        } else {
            Rights::None
        })
    }
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(&*LOGGER_ENV)
        // .pretty()
        .init();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        prev_hook(panic_info);
    }));

    tracing::info!("Doggy-Watch v{VERSION}");
    tracing::info!("{:?}", *ADMINISTRATORS);
    let bot = Bot::new(&*TOKEN);

    let db: DatabaseConnection = Database::connect(&*DATABASE_URL).await?;

    // teloxide::repl(bot, answer).await;
    let state = Arc::new(AppState {db, administrators: (&ADMINISTRATORS).to_vec(), cooldown: DashMap::new()});

    // let handler = dptree::entry()
    //     .branch(Update::filter_message().endpoint(answer))
    //     .branch(Update::filter_callback_query().endpoint(callback_handler));
    let handler = dptree::entry()
        .branch(Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<DialogueState>, DialogueState>()
            .branch(dptree::case![DialogueState::Nothing]
                .branch(
                    dptree::filter_async(is_moderator)
                    .branch(dptree::filter(|msg: Message| {
                        if let Some(text) = msg.text() {
                            recognise_vid(text).is_some()
                            // проверяем что из сообщения можно достать vid (example: /123 where 123 is vid)
                        } else {
                            false
                        }
                    }).endpoint(info))
                    .filter_command::<Command>()
                    .endpoint(answer)
                )
                .branch(
                    dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(insufficient_rights)
                )
                .branch(
                    dptree::filter(|msg: Message| {
                        msg.text().is_some() && msg.from.is_some()
                    })
                    .endpoint(normal_answer)
                ))
            .branch(dptree::case![DialogueState::NewModeratorInput].endpoint(add_moderator_from_recived_message))
            // .branch(dptree::case![DialogueState::RemoveModeratorConfirm { uid }].endpoint(remove_moderator))
        )
        .branch(Update::filter_callback_query()
            .enter_dialogue::<CallbackQuery, InMemStorage<DialogueState>, DialogueState>()
            .branch(dptree::case![DialogueState::Nothing].endpoint(change_status))
            .branch(dptree::case![DialogueState::RemoveModeratorConfirm { uid }].endpoint(remove_moderator))
            .branch(dptree::case![DialogueState::AcceptVideo { accept }].endpoint(accept_video))
            .branch(dptree::case![DialogueState::NewModeratorInput].endpoint(cancel))
            // .endpoint(callback_handler)
        );
    
    Dispatcher::builder(bot, handler)
        // Pass the shared state to the handler as a dependency.
        .dependencies(dptree::deps![state, InMemStorage::<DialogueState>::new()])
        .default_handler(|upd| async move {
            tracing::warn!("Unhandled update: {:?}", upd);
        })
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

async fn is_moderator(state: Arc<AppState>, msg: Message) -> bool {
    if let Some(user) = msg.from {
        let rights = state.check_rights(&user.id).await;
        if let Ok(rights) = rights {
            rights.into()
        } else {
            false
        }
    } else {
        false
    }
}

type MyDialogue = Dialogue<DialogueState, InMemStorage<DialogueState>>;

#[derive(Clone, Default)]
pub enum DialogueState {
    #[default]
    Nothing,
    // User
    AcceptVideo{ accept: ForAccept },
    // Moderator
    NewModeratorInput,
    RemoveModeratorConfirm{uid: String},
}

#[derive(Clone)]
pub struct ForAccept {
    pub ytid: String,
    pub uid: u64,
    pub title: String,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Список поддерживаемых команд:")]
enum Command {
    #[command(description = "отобразить этот текст.")]
    Help,
    #[command(description = "запустить бота.")]
    Start,
    #[command(description = "вывести список.")]
    List,
    // #[command(description = "информация о видео.")]
    // I(i32),
    // #[command(description = "вывести чёрный список.")]
    // Blacklisted,
    #[command(description = "добавить в чёрный список.")]
    Ban(i32),
    // #[command(description = "удалить из чёрного списка.")]
    // RemBlacklisted(u128),
    #[command(description = "вывести список модераторов.")]
    Mods,
    #[command(description = "добавить модератора.")]
    AddMod,
    #[command(description = "удалить модератора.")]
    RemMod(String),
    About
}

async fn answer(bot: Bot, msg: Message, cmd: Command, state: Arc<AppState>, dialogue: MyDialogue) -> anyhow::Result<()> {
    let user = msg.from.unwrap(); // Потому что уже уверены что пользователь администратор или модератор
    let rights = state.check_rights(&user.id).await.unwrap();
    tracing::info!("{rights:?}");
    match cmd {
        Command::Help => {
            let mut result = String::from(&Command::descriptions().to_string());
            result.push_str("\n\nЧтобы получить информацию о видео или изменить его статус просто отправь его номер в чат.");
            bot.send_message(msg.chat.id, result).await?;
        },
        Command::Start => {
            let mut result = String::from(&Command::descriptions().to_string());
            result.push_str("\n\nЧтобы получить информацию о видео или изменить его статус просто отправь его номер в чат.");
            bot.send_message(msg.chat.id, result).await?;
        },
        Command::List => {
            use entity::{actions, videos, sea_orm_active_enums::Status};
            struct Video {
                id: i32,
                title: String,
                url: String,
                contributors: u64,
            }
            let videos = videos::Entity::find().filter(videos::Column::Status.eq(Status::Pending)).all(&state.db).await?;
            if videos.len() != 0 {
                let mut by_date: IndexMap<Date, Vec<Video>> = IndexMap::new();
                for video in videos {
                    let contributors = actions::Entity::find().filter(actions::Column::Vid.eq(video.id)).count(&state.db).await?;
                    let date = video.created_at.date();
                    let url = format!("{}{}", youtube::DEFAULT_YT, video.yt_id);
                    let title = video.title.replace("/", "/ ");
    
                    if let Some(entry) = by_date.get_mut(&date) {
                        entry.push(Video { id: video.id, title, url, contributors });
                    } else {
                        by_date.insert(date, vec![Video { id: video.id, title, url, contributors }]);
                    };
                }
                by_date.sort_unstable_by(|a, _, c, _| c.cmp(a));
                let mut result = String::new();
                for (date, mut videos) in by_date {
                    if result.is_empty() {
                        result.push_str(&format!("[{}]", date.format("%m.%d")));
                    } else {
                        result.push_str(&format!("\n[{}]", date.format("%m.%d")));
                    }
                    videos.sort_unstable_by(|a, b| a.contributors.cmp(&b.contributors));
                    for video in videos {
                        result.push_str(&format!("\n/{} <a href=\"{}\">📺YT</a> (👀{}) <b>{}</b>\n", video.id, video.url, video.contributors, video.title));
                        // result.push_str(&format!("\n<a href=\"tg://resolve?domain={}&start=info%20{}\">{}.</a> <b>{}</b> <a href=\"{DEFAULT_YT}{}\">YT</a> ({})", me.username.clone().unwrap(), video.id, video.id, video.title, video.url, video.contributors));
                    }
                }
                bot.send_message(msg.chat.id, result).parse_mode(ParseMode::Html).link_preview_options(LinkPreviewOptions { is_disabled: true, url: None, prefer_small_media: false, prefer_large_media: false, show_above_text: false  }).await?;
            } else {
                bot.send_message(msg.chat.id, "Нет видео для просмотра :(").await?;
            }
            // for (date, value) in by_date.sort_unstable_by(|a, b| b.cmp(a)) {
            //     result.push_str(&format!("{}: {}\n", date, value));
            // }

            // let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
            //     vec![InlineKeyboardButton::callback("Hello1", "1")],
            //     vec![InlineKeyboardButton::callback("Hello2", "2"), InlineKeyboardButton::callback("Hello1", "1")],
            //     vec![InlineKeyboardButton::callback("Hello3", "3")],
            // ];
            
            // bot.send_message(msg.chat.id, format!("{messages_total:?}")).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
        },
        // Command::Blacklisted => todo!(),
        Command::Ban(vid) => {
            use entity::{videos::{Entity, ActiveModel}, sea_orm_active_enums::Status};
            let video = Entity::find_by_id(vid).one(&state.db).await?;
            if let Some(model) = video {
                let title = model.title.clone();
                let mut video: ActiveModel = model.into();
                video.status = Set(Status::Banned);
                video.updated_at = Set(Some(Local::now().naive_local()));
                if video.update(&state.db).await.is_ok() {
                    bot.send_message(msg.chat.id, format!("Видео <b>\"{title}\"</b> успешно добавленно в чёрный список!")).parse_mode(ParseMode::Html).await?;
                } else {
                    bot.send_message(msg.chat.id, "Произошла ошибка обновления записи в базе данных!").await?;
                }
            } else {
                bot.send_message(msg.chat.id, "Не найдено.").await?;
            }
        },
        // Command::RemBlacklisted() => todo!(),
        Command::Mods => {
            use entity::moderators::{Entity, Model};
            let columns: Vec<Model> = Entity::find().all(&state.db).await.unwrap();
            if columns.len() != 0 {
                let mut str = String::from("Модераторы:");
                for col in columns {
                    tracing::info!("{col:?}");
                    let uid: u64 = col.uid.parse()?;
                    let name = bot.get_chat_member(ChatId(uid as i64), UserId(uid)).await?.user.full_name();
                    let mention = user_mention(UserId(uid), &name);
                    str.push_str(&format!("\n - {mention}\nНа посту с {}, UID: {uid}", col.created_at.format("%Y-%m-%d %H:%M:%S")));
                };
                tracing::info!("Sending message! {str}");
                bot.send_message(msg.chat.id, str).parse_mode(ParseMode::Html).await?
            } else {
                bot.send_message(msg.chat.id, "Модераторов нет").await?
            };
        },
        Command::AddMod => {
            bot.send_message(msg.chat.id, "Перешлите любое сообщение от человека которого вы хотите добавить как модератора:").reply_markup(inline_cancel()).await?;
            dialogue.update(DialogueState::NewModeratorInput).await?;
        },
        Command::RemMod(uid) => {
            if uid.is_empty() {
                bot.send_message(msg.chat.id, "После команды необходимо указать UID модератора. (/remmod 1234567)").await?;
            } else {
                bot.send_message(msg.chat.id, "Вы уверены что хотите удалить модератора?").reply_markup(inline_yes_or_no()).await?;
                dialogue.update(DialogueState::RemoveModeratorConfirm { uid }).await?;
            }
        },
        Command::About => {
            bot.send_message(msg.chat.id, about_msg(&rights)).await?;
        },
    };
    Ok(())
}

fn recognise_vid(text: &str) -> Option<i32> {
    if let Ok(vid) = text.parse::<i32>() {
        Some(vid)
    } else {
        if let Some(unslash) = text.strip_prefix("/") {
            if let Ok(vid) = unslash.parse::<i32>() {
                Some(vid)
            } else {
                None
            }
        } else {
            None
        }
    }
}

async fn info(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    use entity::{videos, actions};
    use youtube::DEFAULT_YT;
    let vid = recognise_vid(msg.text().unwrap()).unwrap(); // Проверено в dptree
    let col = videos::Entity::find_by_id(vid).one(&state.db).await?;
    if let Some(video) = col {
        // Getting creator from actions
        let creator = actions::Entity::find()
            .filter(actions::Column::Vid.eq(video.id))
            .order_by(actions::Column::Id, Order::Asc)
            .one(&state.db).await?
            .ok_or(anyhow::anyhow!("Can't find creator entry for {video:?}"))?;
        let contributors = actions::Entity::find().filter(actions::Column::Vid.eq(video.id)).count(&state.db).await?;

        let creator_uid = creator.uid.parse()?;
        let name = bot.get_chat_member(ChatId(creator_uid as i64), UserId(creator_uid)).await?.user.full_name();
        let creator_mention = user_mention(UserId(creator_uid), &name);

        let out: String = format!(
            "<a href=\"{DEFAULT_YT}{}\">{}</a>\n\
            Добавлено {creator_mention} (👀{contributors})"
            , video.yt_id, video.title);

        // TODO: УБЕДИТСЯ ЧТО НЕ ТРЕБУЕТСЯ https://docs.rs/teloxide/latest/teloxide/types/struct.LinkPreviewOptions.html
        let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
            vec![InlineKeyboardButton::callback("Просмотрено", format!("viewed {}", video.id)), InlineKeyboardButton::callback("В бан", format!("ban {}", video.id))]
        ];
        bot.send_message(msg.chat.id, out).parse_mode(ParseMode::Html).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
    } else {
        bot.send_message(msg.chat.id, "Не найдено.").await?;
    }
    Ok(())
}

async fn insufficient_rights(bot: Bot, msg: Message, cmd: Command) -> anyhow::Result<()> {
    let rights = Rights::None;
    if let Some(user) = check_subscription(&bot, &msg.from.ok_or(anyhow::anyhow!("Message not from user!"))?.id).await {
        match cmd {
            Command::Start => {
                bot.send_sticker(
                    msg.chat.id, 
                    InputFile::file_id("CAACAgIAAxkBAAECxFlnVeGjr8kRcDNWU30uDII5R1DwNAACKl4AAkxE8UmPev9DDR6RgTYE"))
                    .emoji("🥳")
                    .await?;
                bot.send_message(msg.chat.id, format!(
                    "Приветствую {}!\n\
                    Отправьте в этот чат ссылку на YouTube видео, чтобы предложить его для просмотра!",
                    user.full_name()
                )).await?;
            },
            Command::About => {
                bot.send_message(msg.chat.id, about_msg(&rights)).await?;
            },
            _ => {
                bot.send_message(msg.chat.id, format!(
                    "?"
                )).await?;
            }
        }
    } else {
        bot.send_message(msg.chat.id, format!(
            "Вы не подписаны на Telegram канал!"
        )).await?;
    }
    Ok(())
}

async fn normal_answer(bot: Bot, msg: Message, dialogue: MyDialogue) -> anyhow::Result<()> {
    use youtube::*;
    if let Some(text) = msg.clone().text() {
        if let Some(user) = check_subscription(&bot, &msg.from.ok_or(anyhow::anyhow!("Message not from user!"))?.id).await {
            // Get ready!
            if let Some(ytid) = extract_youtube_video_id(text) {
                let meta = get_video_metadata(&ytid).await?;
                // Post
                bot.send_message(msg.chat.id, format!(
                    "Вы уверены что хотите добавить <b>{}</b>",
                    meta.title
                )).parse_mode(ParseMode::Html).reply_markup(inline_yes_or_no()).await?;
                let accept = ForAccept { ytid, uid: user.id.0, title: meta.title };
                dialogue.update(DialogueState::AcceptVideo { accept }).await?;
            } else {
                bot.send_message(msg.chat.id, "Это не похоже на YouTube видео... Долбоёб").await?; 
            }
        } else { 
            bot.send_message(msg.chat.id, "Вы не подписаны на Telegram канал!").await?; 
        }
    } else {
        bot.send_message(msg.chat.id, "Не-а!").await?; 
    }
    Ok(())
}

fn about_msg(rights: &Rights) -> String {
    format!(
        "Doggy-Watch v{VERSION}\n\
        ____________________\n\
        Debug information:\n\
        Rights level: {rights:?}\n\
        Linked channel: {}\n\
        Cooldown duration: {:?}",
        *CHANNEL, COOLDOWN_DURATION
    )
}

async fn add_moderator_from_recived_message(bot: Bot, msg: Message, state: Arc<AppState>, dialogue: MyDialogue) -> anyhow::Result<()> {
    use entity::moderators::ActiveModel;
    if let Some(user) = msg.forward_from_user() {
        let member = check_subscription(&bot, &user.id).await;
        if let Some(user) = member {
            let now = Local::now().naive_local();
            let model = ActiveModel {
                uid: Set(user.id.0.to_string()),
                created_at: Set(now),
            };
            if model.insert(&state.db).await.is_ok() {
                bot.send_message(msg.chat.id, "Модератор добавлен!").await?;
            } else {
                bot.send_message(msg.chat.id, "Произошла ошибка!\nМожет данный модератор уже добавлен?").await?;
            }
            dialogue.exit().await?;
        } else { bot.send_message(msg.chat.id, "Ошибка! Не подписан на канал!").await?; }
    } else { bot.send_message(msg.chat.id, "Ошибка! Перешлите сообщение!").await?; }
    Ok(())
}

// ------------------------
// INLINE
// ------------------------

fn inline_yes_or_no() -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
        vec![InlineKeyboardButton::callback("Да", "yes"), InlineKeyboardButton::callback("Нет", "no")]
    ];
    InlineKeyboardMarkup::new(keyboard)
}
fn inline_cancel() -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
        vec![InlineKeyboardButton::callback("Отменить", "cancel")]
    ];
    InlineKeyboardMarkup::new(keyboard)
}

async fn change_status(bot: Bot, q: CallbackQuery, state: Arc<AppState>) -> anyhow::Result<()> {
    use entity::{videos::{ActiveModel, Entity}, sea_orm_active_enums::Status};
    bot.answer_callback_query(&q.id).await?;
    if let Some(msg) = q.regular_message() {
        if let Some(data) = q.clone().data {
            // ..
            let data: Vec<&str> = data.split(" ").collect();
            let text = if data.len() == 2 {
                let status = match data[0] {
                    "ban" => {
                        Status::Banned
                    },
                    "viewed" => {
                        Status::Viewed
                    }
                    _ => {
                        anyhow::bail!("Unrecognized status! {data:?}");
                    }
                };
                let vid: i32 = data[1].parse()?;

                let video = Entity::find_by_id(vid).one(&state.db).await?;
                if let Some(model) = video {
                    let title = model.title.clone();
                    let mut video: ActiveModel = model.into();
                    video.status = Set(status);
                    video.updated_at = Set(Some(Local::now().naive_local()));
                    if video.update(&state.db).await.is_ok() {
                        &format!("Статус видео <b>\"{title}\"</b> успешно обновлён!")
                    } else {
                       "Произошла ошибка обновления записи в базе данных!"
                    }
                } else {
                    "Не найдено."
                }
            } else {
                "Ошибка распознавания"
            };
            bot.send_message(msg.chat_id().unwrap(), text).parse_mode(ParseMode::Html).await?;
            // else if let Some(id) = q.inline_message_id {
            //     bot.edit_message_text_inline(id, text).await?;
            // }
        }
    }
    Ok(())
}

async fn remove_moderator(bot: Bot, q: CallbackQuery, state: Arc<AppState>, uid: String, dialogue: MyDialogue) -> anyhow::Result<()> {
    use entity::moderators::Entity;
    bot.answer_callback_query(&q.id).await?;
    if let Some(msg) = q.regular_message() {
        if let Some(data) = q.clone().data {
            let text= if &data == "yes" {
                if Entity::delete_by_id(uid).exec(&state.db).await?.rows_affected != 0 {
                    "Модератор удалён!"
                } else {
                    "Произошла ошибка!\nПо всей видимости такого модератора не существует."
                }
            } else {
                "Раскулачивание модера отменено."
            };
            bot.edit_message_text(msg.chat_id().unwrap(), msg.id, text).await?;
            // else if let Some(id) = q.inline_message_id {
            //     bot.edit_message_text_inline(id, text).await?;
            // }
        }
    }
    dialogue.exit().await?;
    Ok(())
}

async fn accept_video(bot: Bot, q: CallbackQuery, state: Arc<AppState>, accept: ForAccept, dialogue: MyDialogue) -> anyhow::Result<()> {
    use entity::{videos, actions, sea_orm_active_enums::Status};
    bot.answer_callback_query(&q.id).await?;
    if let Some(msg) = q.regular_message() {
        if let Some(data) = q.clone().data {
            let text= if &data == "yes" {
                if let Some(last) = state.cooldown.get(&accept.uid) {
                    if last.elapsed() < COOLDOWN_DURATION {
                        bot.edit_message_text(msg.chat_id().unwrap(), msg.id, "Боже... Ты слишком груб с этим ботом. Остуди пыл.").await?;
                        dialogue.exit().await?;
                        return Ok(());
                    }
                }
                let video = if let Some(video ) = videos::Entity::find().filter(videos::Column::YtId.eq(accept.ytid.clone())).one(&state.db).await? {
                    Ok(video)
                } else {
                    let video= videos::ActiveModel {
                        title: Set(accept.title),
                        yt_id: Set(accept.ytid),
                        created_at: Set(Local::now().naive_local()),
                        status: Set(Status::Pending),
                        ..Default::default()
                    };
                    video.insert(&state.db).await
                };
                if let Ok(video) = video {
                    if let Ok(duplicates) = actions::Entity::find().filter(actions::Column::Uid.eq(accept.uid.to_string())).filter(actions::Column::Vid.eq(video.id)).count(&state.db).await {
                        if duplicates == 0 {
                            let action= actions::ActiveModel {
                                uid: Set(accept.uid.to_string()),
                                vid: Set(video.id),
                                created_at: Set(Local::now().naive_local()),
                                ..Default::default()
                            };
                            if action.insert(&state.db).await.is_ok() {
                                state.cooldown.insert(accept.uid, Instant::now());
                                "Добавлено!"
                            } else {
                                videos::Entity::delete_by_id(video.id).exec(&state.db).await?;
                                "База данных вернула ошибку на этапе создания события!"
                            }
                        } else {
                            "Отправлять одно и тоже видео нельзя!"
                        }
                    } else {
                        "База данных вернула ошибку на этапе проверки дублекатов!"
                    }
                } else {
                    "База данных вернула ошибку на этапе создания видео!"
                }
            } else {
                "Отменено."
            };
            bot.edit_message_text(msg.chat_id().unwrap(), msg.id, text).await?;
            // else if let Some(id) = q.inline_message_id {
            //     bot.edit_message_text_inline(id, text).await?;
            // }
        }
    }
    dialogue.exit().await?;
    Ok(())
}

async fn cancel(bot: Bot, q: CallbackQuery, dialogue: MyDialogue) -> anyhow::Result<()> {
    bot.answer_callback_query(&q.id).await?;
    dialogue.exit().await?;
    Ok(())
}

// ------------------------
// FACE CONTROL
// ------------------------

#[derive(Debug)]
enum Rights {
    None,
    Moderator,
    Administrator
}

impl From<Rights> for bool {
    fn from(value: Rights) -> Self {
        match value {
            Rights::None => false,
            _ => true
        }
    }
}

async fn check_subscription(bot: &Bot, uid: &UserId) -> Option<User> {
    let chat_member = bot
        .get_chat_member(ChatId(*CHANNEL), *uid).send().await;

    match chat_member {
        Ok(member) => {
            let kind = member.kind;
            tracing::debug!("{uid}: {kind:?}");
            if match kind {
                teloxide::types::ChatMemberKind::Owner(_owner) => true,
                teloxide::types::ChatMemberKind::Administrator(_administrator) => true,
                teloxide::types::ChatMemberKind::Member => true,
                teloxide::types::ChatMemberKind::Restricted(_restricted) => true,
                teloxide::types::ChatMemberKind::Left => false,
                teloxide::types::ChatMemberKind::Banned(_banned) => false,
            } {
                Some(member.user)
            } else {
                None
            }
        },
        Err(_) => None,
    }
}
