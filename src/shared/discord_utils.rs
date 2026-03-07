const DISCORD_MAX_LENGTH: usize = 2000;

pub fn split_message(text: &str) -> Vec<&str> {
    if text.len() <= DISCORD_MAX_LENGTH {
        return vec![text];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= DISCORD_MAX_LENGTH {
            chunks.push(remaining);
            break;
        }

        let split_at = remaining[..DISCORD_MAX_LENGTH]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(DISCORD_MAX_LENGTH);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk);
        remaining = rest;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_message_returns_single_chunk() {
        let msg = "Hello, world!";
        let chunks = split_message(msg);
        assert_eq!(chunks, vec!["Hello, world!"]);
    }

    #[test]
    fn empty_message_returns_single_chunk() {
        let chunks = split_message("");
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn exactly_2000_chars_returns_single_chunk() {
        let msg = "a".repeat(2000);
        let chunks = split_message(&msg);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 2000);
    }

    #[test]
    fn over_2000_chars_splits_at_newline() {
        // 1990 chars + \n + 1990 chars = 3981 chars total
        let part = "a".repeat(1990);
        let msg = format!("{}\n{}", part, part);
        let chunks = split_message(&msg);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].len() <= 2000);
        assert!(chunks[1].len() <= 2000);
    }

    #[test]
    fn over_2000_chars_no_newline_splits_at_limit() {
        let msg = "a".repeat(4500);
        let chunks = split_message(&msg);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 2000);
        assert_eq!(chunks[1].len(), 2000);
        assert_eq!(chunks[2].len(), 500);
    }
}
