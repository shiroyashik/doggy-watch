use std::sync::Arc;

use indexmap::IndexMap;
use teloxide::{prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup, LinkPreviewOptions, ParseMode}};
use sea_orm::{prelude::*, Order, QueryOrder};

use database::*;

use crate::AppState;

struct Video {
    id: i32,
    title: String,
    url: String,
    contributors: u64,
    status: String,
}

pub async fn command(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    let videos: Vec<(requests::Model, Option<videos::Model>)> = requests::Entity::find()
        .find_also_related(videos::Entity).filter(videos::Column::Banned.eq(false)).all(&state.db).await?;

    let result = generate_list(videos, &state).await;
    match result {
        Ok(list) => {
            let result = if let Some(list) = list {
                list
            } else {
                "Нет видео для просмотра :(".to_string()
            };

            let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
                vec![InlineKeyboardButton::callback("Непросмотренные", "list_unviewed")],
            ];

            bot.send_message(msg.chat.id, result).parse_mode(ParseMode::Html)
                .link_preview_options(LinkPreviewOptions { 
                    is_disabled: true,
                    url: None,
                    prefer_small_media: false,
                    prefer_large_media: false,
                    show_above_text: false
                }).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
        },
        Err(e) => {
            tracing::error!("{:?}", e);
            bot.send_message(msg.chat.id, "Произошла ошибка!").await?;
        },
    }
    Ok(())
}

pub async fn inline(state: Arc<AppState>, bot: Bot, q: CallbackQuery) -> anyhow::Result<()> {
    bot.answer_callback_query(&q.id).await?;
    let videos: Vec<(requests::Model, Option<videos::Model>)> = requests::Entity::find()
        .find_also_related(videos::Entity).filter(videos::Column::Banned.eq(false)).filter(requests::Column::ViewedAt.is_null()).all(&state.db).await?;
    let result = generate_list(videos, &state).await;
    match result {
        Ok(list) => {
            let result = if let Some(list) = list {
                list
            } else {
                "Нет видео для просмотра :(".to_string()
            };

            let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
                vec![InlineKeyboardButton::callback("Обновить", "list_unviewed")],
            ];

            if let Some(message) = q.regular_message() {
                bot.edit_message_text(message.chat.id, message.id, result).parse_mode(ParseMode::Html)
                    .link_preview_options(LinkPreviewOptions {
                        is_disabled: true,
                        url: None,
                        prefer_small_media: false,
                        prefer_large_media: false,
                        show_above_text: false
                    }).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
            } else if let Some(message_id) = q.inline_message_id {
                bot.edit_message_text_inline(&message_id, result)
                    .parse_mode(ParseMode::Html).disable_web_page_preview(true).reply_markup(InlineKeyboardMarkup::new(keyboard)).await?;
            } else {
                bot.send_message(q.from.id, result).parse_mode(ParseMode::Html)
                    .reply_markup(InlineKeyboardMarkup::new(keyboard))
                    .link_preview_options(LinkPreviewOptions {
                        is_disabled: true,
                        url: None,
                        prefer_small_media: false,
                        prefer_large_media: false,
                        show_above_text: false
                    }).await?;
            }
        },
        Err(e) => {
            tracing::error!("{:?}", e);
            bot.send_message(q.from.id, "Произошла ошибка!").await?;
        },
    }
    Ok(())
}

async fn generate_list(videos: Vec<(requests::Model, Option<videos::Model>)>, state: &AppState) -> anyhow::Result<Option<String>> {
    if videos.is_empty() {
        return Ok(None);
    }
    let mut by_date: IndexMap<Date, Vec<Video>> = IndexMap::new();
    for (request, video) in videos {
        let video = video.unwrap();
        let creator = if let Some(c) = request.find_related(actions::Entity).order_by(actions::Column::Id, Order::Asc).one(&state.db).await? {
            c
        } else {
            anyhow::bail!("Can't find creator for {request:?}");
        };

        let contributors = request.find_related(actions::Entity).count(&state.db).await?;
        let date = creator.created_at.date();
        let url = format!("{}{}", youtube::DEFAULT_YT, video.ytid);

        let viewed_times = archived::Entity::find().filter(archived::Column::Ytid.eq(video.ytid.clone())).filter(archived::Column::ViewedAt.is_not_null()).count(&state.db).await?;
        let archived_times = archived::Entity::find().filter(archived::Column::Ytid.eq(video.ytid)).count(&state.db).await?;

        let mut status = String::new();
        status.push(if request.viewed_at.is_some() {
            '👀'
        } else if viewed_times != 0 {
            '⭐'
        } else if archived_times != 0 {
            '📁'
        } else {
            '🆕'
        });

        if let Some(entry) = by_date.get_mut(&date) {
            entry.push(Video { id: request.id, title: video.title, url, contributors, status });
        } else {
            by_date.insert(date, vec![Video { id: request.id, title: video.title, url, contributors, status }]);
        };
    }
    by_date.sort_unstable_by(|a, _, c, _| c.cmp(a));
    let mut result = String::new();
    for (date, mut videos) in by_date {
        if result.is_empty() {
            result.push_str(&format!("[{}]", date.format("%d.%m")));
        } else {
            result.push_str(&format!("\n[{}]", date.format("%d.%m")));
        }
        // result.push_str(&format!(" {}", videos.len()));
        videos.sort_unstable_by(|a, b| a.contributors.cmp(&b.contributors));
        for video in videos {
            let contributors = if video.contributors != 1 {
                format!("(🙍‍♂️{}) ", video.contributors)
            } else {
                String::new()
            };
            result.push_str(&format!("\n{}/{} <a href=\"{}\">📺YT</a> {}<b>{}</b>", video.status, video.id, video.url, contributors, video.title));
            // result.push_str(&format!("\n<a href=\"tg://resolve?domain={}&start=info%20{}\">{}.</a> <b>{}</b> <a href=\"{DEFAULT_YT}{}\">YT</a> ({})", me.username.clone().unwrap(), video.id, video.id, video.title, video.url, video.contributors));
        }
    }
    // result.push_str(&format!("\nВсего: {}", videos_len));
    Ok(Some(result))
}