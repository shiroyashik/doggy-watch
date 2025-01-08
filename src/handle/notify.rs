use std::sync::Arc;

use sea_orm::{EntityTrait, IntoActiveModel, Set, prelude::*};
use teloxide::{prelude::*, Bot};

use database::*;
use crate::AppState;

/// Invert notify status for moderator
pub async fn command(bot: Bot, msg: Message, uid: UserId, state: Arc<AppState>) -> anyhow::Result<()> {
    let text = if let Some(moder) = moderators::Entity::find_by_id(uid.0 as i32).one(&state.db).await? {
        let moder = match moder.notify {
            true => {
                let mut moder = moder.into_active_model();
                moder.notify = Set(false);
                moder
            },
            false => {
                let mut moder = moder.into_active_model();
                moder.notify = Set(true);
                moder
            },
        };
        let moder = moder.update(&state.db).await?;
        
        if moder.notify {
            "Теперь уведомления <b>включены</b>!".to_string()
        } else {
            "Теперь уведомления <b>отключены</b>!".to_string()
        }
    } else {
        let text = format!("No moderator found for {uid}!");
        tracing::error!(text);
        text
    };

    bot.send_message(msg.chat.id, text).parse_mode(teloxide::types::ParseMode::Html).await?;
    Ok(())
}