use serde_json::Value;
use serenity::all::{Context, GuildId};

pub async fn execute(ctx: &Context, args: &Value) -> String {
    let guild_id_value = args
        .get("guild_id")
        .or_else(|| args.get("query"));
    let guild_id = guild_id_value
        .and_then(Value::as_u64)
        .or_else(|| {
            guild_id_value
                .and_then(Value::as_str)
                .and_then(|id| id.parse::<u64>().ok())
        })
        .map(GuildId::new);

    let Some(guild_id) = guild_id else {
        return "Invalid Guild ID".to_string();
    };

    match guild_id.channels(&ctx.http).await {
        Ok(channels) => {
            let list = channels
                .values()
                .map(|c| format!("{}: {} ({:?})", c.name, c.id, c.kind))
                .collect::<Vec<_>>()
                .join("\n");
            list
        },
        Err(e) => format!("Failed to fetch channels: {}", e),
    }
}