#[derive(Debug, PartialEq, Clone)]
pub enum InlineCommand {
    Ban(i32),
    Pardon(i32),
    View(i32),
    Unview(i32),
    ArchiveViewed,
    ArchiveAll,
    ListUnviewed,
    Cancel,
}

impl InlineCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let mut parts = input.split_whitespace();
        Some(match parts.next()? {
            "ban" => Self::Ban(parts.next()?.parse().ok()?),
            "pardon" => Self::Pardon(parts.next()?.parse().ok()?),
            "view" => Self::View(parts.next()?.parse().ok()?),
            "unview" => Self::Unview(parts.next()?.parse().ok()?),
            "archive_viewed" => Self::ArchiveViewed,
            "archive_all" => Self::ArchiveAll,
            "list_unviewed" => Self::ListUnviewed,
            "cancel" => Self::Cancel,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cancel() {
        let text = "cancel";
        let result = InlineCommand::parse(text);
        assert_eq!(result, Some(InlineCommand::Cancel));
    }

    #[test]
    fn test_parse_ban() {
        let text = "ban 123";
        let result = InlineCommand::parse(text);
        assert_eq!(result, Some(InlineCommand::Ban(123)));
    }
}
