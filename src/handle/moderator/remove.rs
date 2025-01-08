use std::sync::Arc;

use sea_orm::EntityTrait;
use teloxide::prelude::*;

use crate::{markup, AppState, DialogueState, MyDialogue};

pub async fn command(bot: Bot, msg: Message, id: UserId, dialogue: MyDialogue, state: Arc<AppState>, uid: String) -> anyhow::Result<()> {
    // add can_add_mods check
    let moderator = database::moderators::Entity::find_by_id(id.0 as i64).one(&state.db).await?.ok_or(anyhow::anyhow!("Ошибка! Не модератор."))?;
    if !moderator.can_add_mods {
        bot.send_message(msg.chat.id, "Недостаточно прав!").await?;
        return Ok(());
    }
    if uid.is_empty() {
        bot.send_message(msg.chat.id, "После команды необходимо указать UID модератора. (/remmod 1234567)").await?;
    } else {
        bot.send_message(msg.chat.id, "Вы уверены что хотите удалить модератора?").reply_markup(markup::inline_yes_or_no()).await?;
        dialogue.update(DialogueState::RemoveModeratorConfirm { uid }).await?;
    }
    Ok(())
}

/// Второй этап удаления модератора.
pub async fn inline(bot: Bot, q: CallbackQuery, state: Arc<AppState>, uid: String, dialogue: MyDialogue) -> anyhow::Result<()> {
    use database::moderators::Entity;
    bot.answer_callback_query(&q.id).await?;
    if let Some(msg) = q.regular_message() {
        if let Some(data) = q.clone().data {
            let text= if &data == "yes" {
                if let Ok(uid) = uid.parse::<u64>() {
                    if Entity::delete_by_id(uid as i32).exec(&state.db).await?.rows_affected != 0 {
                        "Модератор удалён!"
                    } else {
                        "Произошла ошибка!\nПо всей видимости такого модератора не существует."
                    }
                } else {
                    "Ошибка! Это точно число?"
                }
            } else {
                "Раскулачивание модера отменено."
            };
            bot.edit_message_text(msg.chat.id, msg.id, text).await?;
            // else if let Some(id) = q.inline_message_id {
            //     bot.edit_message_text_inline(id, text).await?;
            // }
        }
    }
    dialogue.exit().await?;
    Ok(())
}