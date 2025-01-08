use std::sync::Arc;

use dptree::{filter, filter_map};
use teloxide::{dispatching::{dialogue::{self, GetChatId, InMemStorage}, HandlerExt, UpdateHandler}, prelude::*, types::User};

use crate::{cancel, AppState, Command, DialogueState, InlineCommand, Rights};

mod moderator;
mod about;
mod add;
mod list;
// mod ban;
mod info;
mod start;
mod archive;
mod notify;

pub fn schema() -> UpdateHandler<anyhow::Error> {
    use dptree::case;
    let moderator_commands = dptree::entry()
        .branch(case![Command::Start].endpoint(start::command_mod))
        .branch(case![Command::List].endpoint(list::command))
        .branch(case![Command::Archive].endpoint(archive::command))
        .branch(case![Command::Mods].endpoint(moderator::list::command))
        .branch(case![Command::AddMod].endpoint(moderator::add::command))
        .branch(case![Command::RemMod(uid)].endpoint(moderator::remove::command))
        .branch(case![Command::Notify].endpoint(notify::command))
        .branch(case![Command::About].endpoint(about::command));

    let user_commands = dptree::entry()
        .branch(case![Command::Start].endpoint(start::command_user))
        .branch(case![Command::About].endpoint(about::command));

    let command_handler = dptree::entry()
        .filter_command::<Command>()
        .branch(case![DialogueState::Nothing]
            .branch(case![Rights::None].branch(user_commands))
            .branch(case![Rights::Moderator { can_add_mods }].branch(moderator_commands.clone()))
        );

    let message_handler = Update::filter_message()
        .filter_map(|msg: Message| {
            msg.from // Get User
        })
        .map(|user: User| {
            user.id // Get UserId
        })
        .filter_map_async(|state: Arc<AppState>, uid: UserId| async move {
            state.check_rights(&uid).await.ok()
        })
        // State handlers
        .branch(case![DialogueState::NewModeratorInput].endpoint(moderator::add::recieved_message))
        .branch(command_handler)
        .branch(
            dptree::filter_map(|msg: Message| {
                if let Some(text) = msg.text() {
                    info::recognise_vid(text)
                    // проверяем что из сообщения можно достать vid (example: /123 where 123 is vid)
                } else {
                    None
                }
            }).endpoint(info::message)
        )
        .branch(
            dptree::filter(|msg: Message| {
                msg.text().is_some() && msg.from.is_some()
            })
            .endpoint(add::message)
        );

    let parsable_callback = dptree::entry()
        .chain(filter_map(|q: CallbackQuery| {
            InlineCommand::parse(&q.data?)
        }))
        .branch(case![InlineCommand::Cancel].endpoint(cancel))
        .branch(filter(|com: InlineCommand| {
            matches!(com, InlineCommand::ArchiveAll | InlineCommand::ArchiveViewed)
        }).endpoint(archive::inline))
        .branch(filter(|com: InlineCommand| {
            matches!(com, InlineCommand::Ban(_) | InlineCommand::Pardon(_) | InlineCommand::View(_) | InlineCommand::Unview(_))
        }).endpoint(info::inline));

    let callback_query_handler = Update::filter_callback_query()
        .filter_map(|q: CallbackQuery| {
            q.regular_message().cloned()
        })
        .filter_map(|q: CallbackQuery| {
            q.chat_id()
        })
        .filter_map(|cid: ChatId| {
            cid.as_user()
        })
        .branch(parsable_callback)
        // FIXME: .branch(case![DialogueState::Nothing].endpoint(info::inline))
        .branch(case![DialogueState::RemoveModeratorConfirm { uid }].endpoint(moderator::remove::inline))
        .branch(case![DialogueState::AcceptVideo { ytid, uid, title }].endpoint(add::inline));

    dialogue::enter::<Update, InMemStorage<DialogueState>, DialogueState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}