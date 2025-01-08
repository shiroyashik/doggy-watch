use std::{sync::Arc, vec};

use sea_orm::{prelude::Expr, EntityTrait, QueryFilter, Set};
use teloxide::{prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode}};

use crate::{AppState, InlineCommand};
use database::{actions, requests, archived};

pub async fn command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
        vec![InlineKeyboardButton::callback("Архивировать просмотренные", "archive_viewed")],
        vec![InlineKeyboardButton::callback("Архивировать всё", "archive_all")]
    ];
    let out = "Выберите действие с архивом:";
    bot.send_message(msg.chat.id, out).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
    Ok(())
}

pub async fn inline(
    bot: Bot,
    q: CallbackQuery,
    chatid: ChatId,
    state: Arc<AppState>,
    command: InlineCommand,
) -> anyhow::Result<()> {
    // Есть только два действия, архивровать просмотренные и архивировать всё
    // database::archived::ActiveModel {
    //     id: todo!(),            / Auto
    //     ytid: todo!(),          / From request
    //     viewed_at: todo!(),     / From request
    //     created_by: todo!(),    / From actions
    //     created_at: todo!(),    / Auto
    //     contributors: todo!(),  / From actions
    // }
    bot.answer_callback_query(&q.id).await?;
    let text = match command {
        InlineCommand::ArchiveViewed => {
            match collect_viewed(&state).await {
                Ok(total) => {
                    &format!("<b>\"{}\"</b> просмотренных запросов успешно архивировано!", total)
                },
                Err(err) => {
                    tracing::error!("Caused an exception in archive viewed due: {err:?}");
                    &format!("{err:?}")
                },
            }
        },
        InlineCommand::ArchiveAll => {
            match collect_all(&state).await {
                Ok(total) => {
                    &format!("<b>\"{}\"</b> запросов успешно архивировано!", total)
                },
                Err(err) => {
                    tracing::error!("Caused an exception in archive all due: {err:?}");
                    &format!("{err:?}")
                },
            }
        }
        _ => {
            tracing::error!("Unrecognized status! {command:?}");
            "Ошибка распознавания!"
        }
    };

    bot.send_message(chatid, text).parse_mode(ParseMode::Html).await?;
    Ok(())
}

//
// Auxiliary functions
//

async fn archive(entities: Vec<(requests::Model, Vec<actions::Model>)>, state: &AppState) -> anyhow::Result<u32> {
    if entities.is_empty() {
        anyhow::bail!("Нет объектов для архивации!");
    }

    let mut active_entities = Vec::new();
    for (request, actions) in entities.iter() {
        let creator = actions.iter()
            .min_by_key(|actions| actions.id)
            .ok_or(anyhow::anyhow!("Actions vector cannot be empty!"))?;
        let contributors = actions.len().try_into()?;
        let ytid = request.ytid.clone();
        let viewed_at = request.viewed_at;
        let created_by = creator.uid;
        // let created_at = creator.created_at.clone(); Время архивации, а не создания запроса
        active_entities.push(archived::ActiveModel {
            ytid: Set(ytid),
            viewed_at: Set(viewed_at),
            created_by: Set(created_by),
            // created_at: Set(created_at),
            contributors: Set(contributors),
            ..Default::default()
        });
    }

    let total = active_entities.len().try_into()?;

    archived::Entity::insert_many(active_entities)
        .exec(&state.db)
        .await?;

    Ok(total)
}

async fn collect_viewed(state: &AppState) -> anyhow::Result<u32> {
    let entities: Vec<(requests::Model, Vec<actions::Model>)> = requests::Entity::find()
        .find_with_related(actions::Entity)
        .filter(Expr::col(requests::Column::ViewedAt).is_not_null())
        .all(&state.db)
        .await?;
    let total = archive(entities, state).await?;
    requests::Entity::delete_many()
        .filter(Expr::col(requests::Column::ViewedAt).is_not_null())
        .exec(&state.db)
        .await?;
    Ok(total)

}

async fn collect_all(state: &AppState) -> anyhow::Result<u32> {
    let entities: Vec<(requests::Model, Vec<actions::Model>)> = requests::Entity::find()
        .find_with_related(actions::Entity)
        .all(&state.db)
        .await?;
    let total = archive(entities, state).await?;
    requests::Entity::delete_many()
        .exec(&state.db)
        .await?;
    Ok(total)
}