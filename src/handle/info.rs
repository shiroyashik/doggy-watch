use std::sync::Arc;

use chrono::Local;
use teloxide::{prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode}, utils::html::user_mention};
use sea_orm::{prelude::*, IntoActiveModel, Order, QueryOrder as _, Set};

use crate::{AppState, InlineCommand};
use database::*;
use youtube::DEFAULT_YT;

// Вытаскивает VID из сообщений: /123 или 123
pub fn recognise_vid(text: &str) -> Option<i32> {
    if let Ok(vid) = text.parse::<i32>() {
        Some(vid)
    } else if let Some(unslash) = text.strip_prefix("/") {
        if let Ok(vid) = unslash.parse::<i32>() {
            Some(vid)
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn message(bot: Bot, msg: Message, state: Arc<AppState>, rid: i32) -> anyhow::Result<()> {
    match collect_info(&rid, &state).await {
        Ok((video, request, creator, contributors)) => {
            let name = bot.get_chat_member(ChatId(creator.uid), UserId(creator.uid as u64)).await?.user.full_name();
            let creator_mention = user_mention(UserId(creator.uid as u64), &name);

            let out: String = format!(
                "<a href=\"{DEFAULT_YT}{}\">{}</a>\n\
                Добавлено {creator_mention} (👀{contributors})"
                , video.ytid, video.title);

            // TODO: УБЕДИТСЯ ЧТО НЕ ТРЕБУЕТСЯ https://docs.rs/teloxide/latest/teloxide/types/struct.LinkPreviewOptions.html
            let ban_title = if video.banned {
                ("Пардоньте", "pardon")
            } else {
                ("В бан", "ban")
            };
            let viewed_title = if request.viewed_at.is_some() {
                ("Убрать из просмотренных", "unview")
            } else {
                ("В просмотренные", "view")
            };
            let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
                vec![
                    InlineKeyboardButton::callback(viewed_title.0, format!("{} {}", viewed_title.1, request.id)),
                    InlineKeyboardButton::callback(ban_title.0, format!("{} {}", ban_title.1, request.id))
                ]
            ];
            bot.send_message(msg.chat.id, out).parse_mode(ParseMode::Html).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
        },
        Err(err) => {
            tracing::error!("Caused an exception in collect_info due: {err:?}");
            bot.send_message(msg.chat.id, format!("{err:?}")).await?;
        },
    }
    Ok(())
}

async fn collect_info(rid: &i32, state: &AppState) -> anyhow::Result<(videos::Model, requests::Model, actions::Model, u64)> {
    let request = requests::Entity::find_by_id(*rid).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find request ID {rid}"))?;

    let video = request.find_related(videos::Entity)
        .one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find video entry for {request:?}"))?;
    let creator = request.find_related(actions::Entity)
        .order_by(actions::Column::Id, Order::Asc)
        .one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find creator entry for {request:?}"))?;

    let contributors = request.find_related(actions::Entity).count(&state.db).await?;

    Ok((video, request, creator, contributors))
}

// Изменение статуса видео.
pub async fn inline(bot: Bot, q: CallbackQuery, chatid: ChatId, state: Arc<AppState>, command: InlineCommand) -> anyhow::Result<()> {
    bot.answer_callback_query(&q.id).await?;

    let text = {
        match command {
            InlineCommand::Ban(rid) => {
                match ban(&rid, &state).await {
                    Ok(vid) => {
                        &format!("Статус видео <b>\"{}\"</b> успешно обновлён!", vid.title)
                    },
                    Err(err) => {
                        tracing::error!("Caused an exception in ban due: {err:?}");
                        &format!("{err:?}")
                    },
                }
            },
            InlineCommand::Pardon(rid) => {
                match pardon(&rid, &state).await {
                    Ok(vid) => {
                        &format!("Статус видео <b>\"{}\"</b> успешно обновлён!", vid.title)
                    },
                    Err(err) => {
                        tracing::error!("Caused an exception in pardon due: {err:?}");
                        &format!("{err:?}")
                    },
                }
            },
            InlineCommand::View(rid) => {
                match view(&rid, &state).await {
                    Ok(vid) => {
                        &format!("Статус видео <b>\"{}\"</b> успешно обновлён!", vid.title)
                    },
                    Err(err) => {
                        tracing::error!("Caused an exception in view due: {err:?}");
                        &format!("{err:?}")
                    },
                }
            },
            InlineCommand::Unview(rid) => {
                match unview(&rid, &state).await {
                    Ok(vid) => {
                        &format!("Статус видео <b>\"{}\"</b> успешно обновлён!", vid.title)
                    },
                    Err(err) => {
                        tracing::error!("Caused an exception in unview due: {err:?}");
                        &format!("{err:?}")
                    },
                }
            },
            _ => {
                tracing::error!("Unrecognized status! {command:?}");
                "Ошибка распознавания!"
            }
        }
    };
    bot.send_message(chatid, text).parse_mode(ParseMode::Html).await?;
    Ok(())
}

// Auxiliary functions

async fn ban(rid: &i32, state: &AppState) -> anyhow::Result<videos::Model> {
    let request = requests::Entity::find_by_id(*rid).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find request ID {rid}"))?;
    let mut video = request.find_related(videos::Entity).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find video for {request:?}"))?.into_active_model();
    video.banned = Set(true);
    Ok(video.update(&state.db).await?)
}

async fn view(rid: &i32, state: &AppState) -> anyhow::Result<videos::Model> {
    let mut request = requests::Entity::find_by_id(*rid).one(&state.db).await?
    .ok_or(anyhow::anyhow!("Can't find request ID {rid}"))?.into_active_model();
request.viewed_at = Set(Some(Local::now().naive_local()));
request.update(&state.db).await?.find_related(videos::Entity).one(&state.db).await?
.ok_or(anyhow::anyhow!("Can't find video by RID {rid}"))
}

// Alternate

async fn pardon(rid: &i32, state: &AppState) -> anyhow::Result<videos::Model> {
    let request = requests::Entity::find_by_id(*rid).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find request ID {rid}"))?;
    let mut video = request.find_related(videos::Entity).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find video for {request:?}"))?.into_active_model();
    video.banned = Set(false);
    Ok(video.update(&state.db).await?)
}

async fn unview(rid: &i32, state: &AppState) -> anyhow::Result<videos::Model> {
    let mut request = requests::Entity::find_by_id(*rid).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find request ID {rid}"))?.into_active_model();
    request.viewed_at = Set(None);
    request.update(&state.db).await?.find_related(videos::Entity).one(&state.db).await?
        .ok_or(anyhow::anyhow!("Can't find video by RID {rid}"))
}