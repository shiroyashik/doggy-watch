use chrono::Local;
use teloxide::{prelude::*, Bot};

use crate::{Rights, CHANNEL, COOLDOWN_DURATION, VERSION};

pub async fn command(bot: Bot, msg: Message, rights: Rights) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, format!(
            "Doggy-Watch v{VERSION}\n\
            ____________________\n\
            Debug information:\n\
            Rights level: {rights:?}\n\
            Linked channel: {}\n\
            Cooldown duration: {:?}\n\
            Server time:\n\
            {}",
            *CHANNEL, COOLDOWN_DURATION,
            Local::now().format("%Y-%m-%d %H:%M:%S")
        )).await?;
    Ok(())
}