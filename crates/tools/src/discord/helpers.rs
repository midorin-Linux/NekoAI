use std::str::FromStr;

use serde::Serialize;
use serde_json::{Value, json};
use serenity::all::{
    AutoArchiveDuration, ChannelId, ChannelType, Colour, GuildId, MessageId, ReactionType,
    ScheduledEventStatus, ScheduledEventType, Timestamp, UserId,
};

pub fn ok(data: Value) -> Value {
    json!({ "ok": true, "data": data })
}

pub fn err(message: impl ToString) -> Value {
    json!({ "ok": false, "error": message.to_string() })
}

pub fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or_else(|_| json!({ "error": "serialization_failed" }))
}

pub fn parse_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    }
}

pub fn parse_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|v| u32::try_from(v).ok()),
        Value::String(value) => value.parse::<u32>().ok(),
        _ => None,
    }
}

pub fn parse_u16(value: &Value) -> Option<u16> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|v| u16::try_from(v).ok()),
        Value::String(value) => value.parse::<u16>().ok(),
        _ => None,
    }
}

pub fn parse_u8(value: &Value) -> Option<u8> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|v| u8::try_from(v).ok()),
        Value::String(value) => value.parse::<u8>().ok(),
        _ => None,
    }
}

pub fn parse_bool(value: &Value) -> Option<bool> {
    value.as_bool()
}

pub fn parse_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.to_string()),
        Value::Number(number) => number.as_u64().map(|v| v.to_string()),
        _ => None,
    }
}

pub fn parse_u64_list(value: &Value) -> Option<Vec<u64>> {
    let items = value.as_array()?;
    let parsed = items.iter().filter_map(parse_u64).collect::<Vec<_>>();
    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

pub fn get_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(parse_u64)
}

pub fn get_u32(args: &Value, key: &str) -> Option<u32> {
    args.get(key).and_then(parse_u32)
}

pub fn get_u16(args: &Value, key: &str) -> Option<u16> {
    args.get(key).and_then(parse_u16)
}

pub fn get_u8(args: &Value, key: &str) -> Option<u8> {
    args.get(key).and_then(parse_u8)
}

pub fn get_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(parse_bool)
}

pub fn get_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(parse_string)
}

pub fn get_u64_list(args: &Value, key: &str) -> Option<Vec<u64>> {
    args.get(key).and_then(parse_u64_list)
}

pub fn get_guild_id_default(args: &Value) -> Option<GuildId> {
    get_u64(args, "guild_id").map(GuildId::new)
}

pub fn get_channel_id(args: &Value, key: &str) -> Option<ChannelId> {
    get_u64(args, key).map(ChannelId::new)
}

pub fn get_user_id(args: &Value, key: &str) -> Option<UserId> {
    get_u64(args, key).map(UserId::new)
}

pub fn get_message_id(args: &Value, key: &str) -> Option<MessageId> {
    get_u64(args, key).map(MessageId::new)
}

pub fn parse_channel_type(value: &Value) -> Option<ChannelType> {
    let raw = value.as_str()?.trim().to_lowercase();
    match raw.as_str() {
        "text" | "guild_text" => Some(ChannelType::Text),
        "voice" | "guild_voice" => Some(ChannelType::Voice),
        "category" | "guild_category" => Some(ChannelType::Category),
        "news" | "announcement" | "guild_news" => Some(ChannelType::News),
        "stage" | "stage_voice" => Some(ChannelType::Stage),
        "forum" | "guild_forum" => Some(ChannelType::Forum),
        "public_thread" => Some(ChannelType::PublicThread),
        "private_thread" => Some(ChannelType::PrivateThread),
        "news_thread" => Some(ChannelType::NewsThread),
        _ => None,
    }
}

pub fn parse_thread_type(value: &Value) -> Option<ChannelType> {
    let raw = value.as_str()?.trim().to_lowercase();
    match raw.as_str() {
        "public" | "public_thread" => Some(ChannelType::PublicThread),
        "private" | "private_thread" => Some(ChannelType::PrivateThread),
        "news" | "news_thread" => Some(ChannelType::NewsThread),
        _ => None,
    }
}

pub fn parse_auto_archive_duration(value: &Value) -> Option<AutoArchiveDuration> {
    let minutes = parse_u64(value)?;
    match minutes {
        60 => Some(AutoArchiveDuration::OneHour),
        1440 => Some(AutoArchiveDuration::OneDay),
        4320 => Some(AutoArchiveDuration::ThreeDays),
        10080 => Some(AutoArchiveDuration::OneWeek),
        _ => None,
    }
}

pub fn parse_scheduled_event_type(value: &Value) -> Option<ScheduledEventType> {
    let raw = value.as_str()?.trim().to_lowercase();
    match raw.as_str() {
        "voice" => Some(ScheduledEventType::Voice),
        "stage" | "stage_instance" => Some(ScheduledEventType::StageInstance),
        "external" => Some(ScheduledEventType::External),
        _ => None,
    }
}

pub fn parse_scheduled_event_status(value: &Value) -> Option<ScheduledEventStatus> {
    let raw = value.as_str()?.trim().to_lowercase();
    match raw.as_str() {
        "scheduled" => Some(ScheduledEventStatus::Scheduled),
        "active" => Some(ScheduledEventStatus::Active),
        "completed" => Some(ScheduledEventStatus::Completed),
        "canceled" | "cancelled" => Some(ScheduledEventStatus::Canceled),
        _ => None,
    }
}

pub fn parse_timestamp(value: &Value) -> Option<Timestamp> {
    value
        .as_str()
        .and_then(|value| Timestamp::parse(value).ok())
}

pub fn parse_colour(value: &Value) -> Option<Colour> {
    match value {
        Value::Number(number) => number.as_u64().map(|value| Colour::new(value as u32)),
        Value::String(value) => {
            let raw = value.trim_start_matches('#');
            u32::from_str_radix(raw, 16).ok().map(Colour::new)
        }
        _ => None,
    }
}

pub fn parse_reaction_type(value: &Value) -> Option<ReactionType> {
    parse_string(value).and_then(|value| ReactionType::from_str(&value).ok())
}
