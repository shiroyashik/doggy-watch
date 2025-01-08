use std::sync::Arc;

use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use teloxide::prelude::*;

use crate::{check_subscription, markup, AppState, DialogueState, MyDialogue};

pub async fn command(bot: Bot, msg: Message, dialogue: MyDialogue) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Перешлите любое сообщение от человека которого вы хотите добавить как модератора:").reply_markup(markup::inline_cancel()).await?;
    dialogue.update(DialogueState::NewModeratorInput).await?;
    Ok(())
}

/// Второй этап добавления модератора.
/// Захватывается пересланное сообщение от другого пользователя и из него достаётся UserId.
pub async fn recieved_message(bot: Bot, msg: Message, id: UserId, state: Arc<AppState>, dialogue: MyDialogue) -> anyhow::Result<()> {
    use database::moderators;
    // add can_add_mods check
    let moderator = moderators::Entity::find_by_id(id.0 as i64).one(&state.db).await?.ok_or(anyhow::anyhow!("Ошибка! Не модератор."))?;
    if !moderator.can_add_mods {
        bot.send_message(msg.chat.id, "Недостаточно прав!").await?;
        dialogue.exit().await?;
        return Ok(());
    }
    // if let Some(current) =  
    if let Some(user) = msg.forward_from_user() {
        let member = check_subscription(&bot, &user.id).await;
        if let Some(user) = member {
            let model = moderators::ActiveModel {
                id: Set(user.id.0 as i64),
                ..Default::default()
            };
            if model.insert(&state.db).await.is_ok() {
                bot.send_message(msg.chat.id, "Модератор добавлен!").await?;
            } else {
                bot.send_message(msg.chat.id, "Произошла ошибка!\nМожет данный модератор уже добавлен?").await?;
            }
            dialogue.exit().await?;
        } else { bot.send_message(msg.chat.id, "Ошибка! Не подписан на канал!").await?; }
    } else { bot.send_message(msg.chat.id, "Ошибка! Перешлите сообщение!").await?; }
    dialogue.exit().await?;
    Ok(())
}