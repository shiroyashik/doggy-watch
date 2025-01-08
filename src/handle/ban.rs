use teloxide::prelude::*;

// FIXME: После редизайна код был перемещён без изменений!

async fn command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    use database::{videos::{Entity, ActiveModel}, sea_orm_active_enums::Status};
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
    Ok(())
}