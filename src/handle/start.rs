use teloxide::{prelude::*, types::{InputFile, User}, utils::command::BotCommands as _};

use crate::Command;

pub async fn command_user(bot: Bot, msg: Message, user: User) -> anyhow::Result<()> {
    bot.send_sticker(
            msg.chat.id, 
            InputFile::file_id("CAACAgIAAxkBAAECxFlnVeGjr8kRcDNWU30uDII5R1DwNAACKl4AAkxE8UmPev9DDR6RgTYE")
        ).emoji("🥳")
        .await?;
    bot.send_message(msg.chat.id, format!(
            "Приветствую {}!\n\
            Отправьте в этот чат ссылку на YouTube видео, чтобы предложить его для просмотра!",
            user.full_name()
        )).await?;
    Ok(())
}

pub async fn command_mod(bot: Bot, msg: Message) -> anyhow::Result<()> {
    let mut result = String::from(&Command::descriptions().to_string());
    result.push_str("\n\nЧтобы получить информацию о видео или изменить его статус просто отправь его номер в чат.");
    bot.send_message(msg.chat.id, result).await?;
    Ok(())
}