use std::sync::Arc;

use sea_orm::EntityTrait as _;
use teloxide::{prelude::*, types::ParseMode, utils::html::user_mention};

use crate::AppState;

pub async fn command(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    use database::moderators::{Entity, Model};
    let columns: Vec<Model> = Entity::find().all(&state.db).await.unwrap();
    if !columns.is_empty() {
        let mut str = String::from("Модераторы:");
        for col in columns {
            tracing::info!("{col:?}");
            let uid: u64 = col.id as u64;
            let name = bot.get_chat_member(ChatId(col.id), UserId(uid)).await?.user.full_name();
            let mention = user_mention(UserId(uid), &name);
            str.push_str(&format!("\n - {mention}\nНа посту с {}, UID: {uid}", col.created_at.format("%Y-%m-%d %H:%M:%S")));
        };
        tracing::info!("Sending message! {str}");
        bot.send_message(msg.chat.id, str).parse_mode(ParseMode::Html).await?
    } else {
        bot.send_message(msg.chat.id, "Модераторов нет").await?
    };
    Ok(())
}