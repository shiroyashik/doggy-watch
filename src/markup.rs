use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn inline_yes_or_no() -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
        vec![InlineKeyboardButton::callback("Да", "yes"), InlineKeyboardButton::callback("Нет", "no")]
    ];
    InlineKeyboardMarkup::new(keyboard)
}
pub fn inline_cancel() -> InlineKeyboardMarkup {
    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![
        vec![InlineKeyboardButton::callback("Отменить", "cancel")]
    ];
    InlineKeyboardMarkup::new(keyboard)
}