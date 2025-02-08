use std::sync::Arc;

use database::*;
use sea_orm::{prelude::*, EntityTrait, IntoActiveModel, Set};
use teloxide::{prelude::*, types::ParseMode};
use tokio::time::Instant;

use crate::{check_subscription, markup, notify, AppState, DialogueState, MyDialogue, CHANNEL_INVITE_HASH, COOLDOWN_DURATION};

pub async fn message(bot: Bot, msg: Message, dialogue: MyDialogue) -> anyhow::Result<()> {
    use youtube::*;
    if let Some(text) = msg.clone().text() {
        if let Some(user) = check_subscription(&bot, &msg.clone().from.ok_or(anyhow::anyhow!("Message not from user!"))?.id).await {
            // Get ready!
            if let Some(ytid) = extract_youtube_video_id(text) {
                let meta = match get_video_metadata(&ytid).await {
                    Ok(meta) => meta,
                    Err(err) => {
                        tracing::error!("Caused an exception in get_video_metadata due: {err:?}");
                        bot.send_message(msg.chat.id, "Ошибка при получении метаданных видео!").await?;
                        return Ok(());
                    },
                };
                // Post
                bot.send_message(msg.chat.id, format!(
                    "Вы уверены что хотите добавить <b>{}</b>",
                    meta.title
                )).parse_mode(ParseMode::Html).reply_markup(markup::inline_yes_or_no()).await?;
                dialogue.update(DialogueState::AcceptVideo { ytid, uid: user.id.0, title: meta.title }).await?;
            } else {
                tracing::debug!("Not a YouTube video: {:?}", msg);
                bot.send_message(msg.chat.id, "Это не похоже на YouTube видео... Долбоёб").await?;
            }
        } else {
            let link = if let Some(hash) = CHANNEL_INVITE_HASH.as_ref() {
                &format!("<a href=\"tg://join?invite={}\">Telegram канал</a>", hash)
            } else {
                "Telegram канал"
            };
            bot.send_message(msg.chat.id, format!("Вы не подписаны на {}!", link)).parse_mode(ParseMode::Html).await?; 
        }
    } else {
        bot.send_message(msg.chat.id, "Не-а!").await?; 
    }
    Ok(())
}

pub async fn inline(
    bot: Bot,
    q: CallbackQuery,
    msg: Message,
    state: Arc<AppState>,
    (ytid, uid, title): (String, u64, String),
    dialogue: MyDialogue
) -> anyhow::Result<()> {
    let data = q.data.ok_or(anyhow::anyhow!("Inline: Нет данных!"))?;
    let text = if &data == "yes" {
        if let Some(last) = state.cooldown.get(&uid) {
            if last.elapsed() < COOLDOWN_DURATION {
                bot.edit_message_text(msg.chat.id, msg.id, "Слишком часто!").await?;
                dialogue.exit().await?;
                return Ok(());
            }
        }
        match add_video(&ytid, &title, &state).await {
            Ok(col) => {
                // Теперь видео создано. Можно приступать к созданию "запроса" и действия
                match add_action(&col, uid, &state).await {
                    Ok(_) => {
                        // Обновляем кул-давн.
                        state.cooldown.insert(uid, Instant::now());
                        // Обновляем данные о пользователе
                        if let Err(err) = add_user(uid, &state).await {
                            tracing::error!("Caused an exception in add_user due: {err:?}");
                        }
                        // Отправляем уведомления
                        let bot_clone = bot.clone();
                        tokio::spawn(async move {
                            let _ = notify(&bot_clone, format!("Добавленно новое видео: <b>{title}</b>!"), &state, vec![UserId(uid)]).await.inspect_err(|err| {
                                tracing::error!("Caused an exception in notify due: {err:?}");
                            });
                        });
                        "Добавлено!"
                    },
                    Err(err) => {
                        tracing::error!("Caused an exception in add_action due: {err:?}");
                        &format!("{err:?}")
                    },
                }
            },
            Err(err) => {
                tracing::error!("Caused an exception in add_video due: {err:?}");
                &format!("{err:?}")
            },
        }
    } else {
        "Отменено."
    };
    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    dialogue.exit().await?;
    Ok(())
}



async fn add_video(ytid: &str, title: &str, state: &AppState) -> anyhow::Result<videos::Model> {
    // Проверяем есть ли необходимость в создании столбца video
    if let Some(video) = videos::Entity::find_by_id(ytid).one(&state.db).await? {
        // Необходимо проверить заблокировано ли видео и создавался ли запрос для этого видео
        if video.banned {
            anyhow::bail!("Ошибка: В чёрном списке!\nВероятнее всего был неоднократно просмотрен.")
        }
        Ok(video)
    } else {
        let new = videos::ActiveModel {
            ytid: Set(ytid.to_string()),
            title: Set(title.to_string()),
            ..Default::default()
        };
        Ok(new.insert(&state.db).await?)
    }
}

async fn add_action(col: &videos::Model, uid: u64, state: &AppState) -> anyhow::Result<()> {
    // Проверяем существует ли запрос
    let req = if let Some(req_col) = col.find_related(requests::Entity).one(&state.db).await? {
        // Запрос существует
        // Проверяем был ли уже просмотрен
        if req_col.viewed_at.is_some() {
            anyhow::bail!("Ошибка: Просмотрено!\nВидео было отмечано как просмотренное {}", req_col.viewed_at.unwrap().format("%Y-%m-%d %H:%M:%S"))
        }
        // Проверяем внёс ли этот пользователь свой "вклад" в этот запрос
        if 0 != req_col.find_related(actions::Entity).filter(actions::Column::Uid.eq(uid)).count(&state.db).await? {
            // Пользователь сделал свой "вклад", больше одного нельзя
            anyhow::bail!("Ошибка: Такой запрос уже существует!\nВы уже запрашивали данное видео ранее.")
        }
        req_col
    } else {
        // Запрос не существует, создаём...
        let new_req = requests::ActiveModel {
            ytid: Set(col.ytid.clone()),
            ..Default::default()
        };
        new_req.insert(&state.db).await?
    };

    let new_act = actions::ActiveModel {
        rid: Set(req.id),
        uid: Set(uid as i64),
        ..Default::default()
    };
    
    // Обрабатываем ошибку на случай неудачи, чтобы удалить запрос без действия
    match new_act.insert(&state.db).await {
        Ok(_) => Ok(()),
        Err(err) => {
            // Если для запроса не существует "действий", удаляем его.
            if 0 == req.find_related(actions::Entity).count(&state.db).await? {
                req.delete(&state.db).await?;
            }
            Err(err.into())
        },
    }
}

async fn add_user(uid: u64, state: &AppState) -> anyhow::Result<users::Model> {
    if let Some(user) = users::Entity::find_by_id(uid as i64).one(&state.db).await? {
        let contributions = user.contributions;
        let mut user = user.into_active_model();
        user.contributions = Set(contributions + 1);
        Ok(user.update(&state.db).await?)
    } else {
        let user = users::ActiveModel {
            id: Set(uid as i64),
            contributions: Set(1),
            ..Default::default()
        };
        Ok(user.insert(&state.db).await?)
    }
}