use std::{env::var, sync::Arc, time::Duration};

use dashmap::DashMap;
use database::moderators;
use migration::{Migrator, MigratorTrait};
use sea_orm::{prelude::*, sea_query::OnConflict, ConnectOptions, Database, Set};
use teloxide::{
    dispatching::dialogue::InMemStorage,
    macros::BotCommands, prelude::*, types::User
};
use tokio::time::Instant;
use tracing_panic::panic_hook;
use lazy_static::lazy_static;

mod handle;
mod markup;

mod inline;
pub use inline::InlineCommand;

pub const COOLDOWN_DURATION: Duration = Duration::from_secs(10);
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref LOGGER_ENV: String = {
        var("RUST_LOG").unwrap_or(String::from("info"))
    };
    pub static ref TOKEN: String = {
        var("TOKEN").expect("TOKEN env not set.")
    };
    pub static ref DATABASE_URL: String = {
        var("DATABASE_URL").expect("DATABASE_URL env not set.")
    };
    pub static ref ADMINISTRATORS: Vec<u64> = {
        var("ADMINISTRATORS").unwrap_or(String::from(""))
        .split(',').filter_map(|s| s.parse().ok()).collect()
    };
    pub static ref CHANNEL: i64 = {
        var("CHANNEL").expect("TOKEN env not set.").parse().expect("Cant't parse env CHANNEL to i64.")
    };
    pub static ref CHANNEL_INVITE_HASH: Option<String> = {
        var("CHANNEL_INVITE_HASH").ok()
    };
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(&*LOGGER_ENV)
        // .pretty()
        .init();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        prev_hook(panic_info);
    }));

    tracing::info!("Doggy-Watch v{VERSION}");
    tracing::info!("{:?}", *ADMINISTRATORS);
    let bot = Bot::new(&*TOKEN);

    let mut opt = ConnectOptions::new(&*DATABASE_URL);
    opt.sqlx_logging_level(tracing::log::LevelFilter::Trace);
    let db: DatabaseConnection = Database::connect(opt).await?;

    // applying migrations
    Migrator::up(&db, None).await?;

    // add administrators to db
    {
        let admins: Vec<moderators::ActiveModel> = ADMINISTRATORS.iter().map(|&x| {
            moderators::ActiveModel {
                id: Set(x as i64),
                can_add_mods: Set(true),
                ..Default::default()
            }
        }).collect();
        moderators::Entity::insert_many(admins)
            .on_conflict(OnConflict::column(moderators::Column::Id)
                .update_column(moderators::Column::CanAddMods).to_owned()
            ).exec(&db).await?;
    }


    // teloxide::repl(bot, answer).await;
    let state = Arc::new(AppState {db, cooldown: DashMap::new()});
    
    Dispatcher::builder(bot, handle::schema())
        // Pass the shared state to the handler as a dependency.
        .dependencies(dptree::deps![state, InMemStorage::<DialogueState>::new()])
        .default_handler(|upd| async move {
            tracing::warn!("Unhandled update: {:?}", upd);
        })
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

pub type MyDialogue = Dialogue<DialogueState, InMemStorage<DialogueState>>;

#[derive(Clone, Default)]
pub enum DialogueState {
    #[default]
    Nothing,
    // User
    AcceptVideo{ ytid: String, uid: u64, title: String },
    // Moderator
    NewModeratorInput,
    RemoveModeratorConfirm{ uid: String },
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Список поддерживаемых команд:")]
enum Command {
    #[command(description = "запустить бота и/или вывести этот текст.")]
    Start,
    #[command(description = "вывести список.")]
    List,
    #[command(description = "действия с архивом.")]
    Archive,
    #[command(description = "вывести список модераторов.")]
    Mods,
    #[command(description = "добавить модератора.")]
    AddMod,
    #[command(description = "удалить модератора.")]
    RemMod(String),
    #[command(description = "включить/выключить уведомления.")]
    Notify,
    About
}

// ------------------------
// NOTIFICATIONS
// ------------------------

async fn notify(bot: &Bot, title: String, state: &AppState, exclude: Vec<UserId>) -> anyhow::Result<()> {
    let notifiable = moderators::Entity::find().filter(moderators::Column::Notify.eq(true)).all(&state.db).await?;
    if notifiable.is_empty() {
        // No one to notify
        return Ok(());
    }
    let mesg = format!("📢 {title}");
    for moder in notifiable {
        let uid = UserId(moder.id as u64);
        if exclude.contains(&uid) {
            continue;
        }
        let chat_id: ChatId = uid.into();
        bot.send_message(chat_id, mesg.clone()).parse_mode(teloxide::types::ParseMode::Html).await?;
    }
    Ok(())
}

// ------------------------
// INLINE
// ------------------------

// FIXME: DEPREACTED: WILL BE REPLACED WITH InlineCommand
pub async fn cancel(bot: Bot, q: CallbackQuery, dialogue: MyDialogue) -> anyhow::Result<()> {
    // FIXME: ADD CHECK FOR CANCEL DATA && 
    bot.answer_callback_query(&q.id).await?;
    dialogue.exit().await?;
    Ok(())
}

// -------------------------
// FACE CONTROL // APP STATE
// -------------------------

struct AppState {
    db: DatabaseConnection,
    cooldown: DashMap<u64, Instant>
    
}

impl AppState {
    /// Возвращает Result<Rights> для переданного пользователя 
    async fn check_rights(&self, uid: &UserId) -> anyhow::Result<Rights> {
        use database::moderators::Entity as Moderators;

        Ok(if let Some(moder) = Moderators::find_by_id(uid.0 as i32).one(&self.db).await? {
            Rights::Moderator { can_add_mods: moder.can_add_mods }
        } else {
            Rights::None
        })
    }
}

#[derive(Debug, Clone)]
enum Rights {
    None,
    Moderator {
        can_add_mods: bool
    },
}

/// Проверка подписки
async fn check_subscription(bot: &Bot, uid: &UserId) -> Option<User> {
    let chat_member = bot
        .get_chat_member(ChatId(*CHANNEL), *uid).send().await;

    match chat_member {
        Ok(member) => {
            let kind = member.kind;
            tracing::debug!("{uid}: {kind:?}");
            if match kind {
                teloxide::types::ChatMemberKind::Owner(_owner) => true,
                teloxide::types::ChatMemberKind::Administrator(_administrator) => true,
                teloxide::types::ChatMemberKind::Member => true,
                teloxide::types::ChatMemberKind::Restricted(_restricted) => true,
                teloxide::types::ChatMemberKind::Left => false,
                teloxide::types::ChatMemberKind::Banned(_banned) => false,
            } {
                Some(member.user)
            } else {
                None
            }
        },
        Err(_) => None,
    }
}
