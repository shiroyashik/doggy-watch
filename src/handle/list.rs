use std::sync::Arc;

use indexmap::IndexMap;
use teloxide::{prelude::*, types::{LinkPreviewOptions, ParseMode}};
use sea_orm::{prelude::*, Order, QueryOrder};

use database::*;

use crate::AppState;

pub async fn command(bot: Bot, msg: Message, state: Arc<AppState>) -> anyhow::Result<()> {
    struct Video {
        id: i32,
        title: String,
        url: String,
        contributors: u64,
        status: String,
    }
    let videos: Vec<(requests::Model, Option<videos::Model>)> = requests::Entity::find()
        .find_also_related(videos::Entity).filter(videos::Column::Banned.eq(false)).all(&state.db).await?;
    // let videos_len = videos.len();
    if !videos.is_empty() {
        let mut by_date: IndexMap<Date, Vec<Video>> = IndexMap::new();
        for (request, video) in videos {
            let video = video.unwrap();
            let creator = if let Some(c) = request.find_related(actions::Entity).order_by(actions::Column::Id, Order::Asc).one(&state.db).await? {
                c
            } else {
                let data = format!("Can't find creator for {request:?}");
                bot.send_message(msg.chat.id, data.clone()).await?;
                anyhow::bail!(data);
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
        bot.send_message(msg.chat.id, result).parse_mode(ParseMode::Html).link_preview_options(LinkPreviewOptions { is_disabled: true, url: None, prefer_small_media: false, prefer_large_media: false, show_above_text: false  }).await?;
    } else {
        bot.send_message(msg.chat.id, "Нет видео для просмотра :(").await?;
    }
    Ok(())
}