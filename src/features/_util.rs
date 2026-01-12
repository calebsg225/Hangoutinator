//! src/features/_util.rs

use serenity::all::{Context, GuildId, GuildInfo, Member};

/// fetch guilds the bot is in
/// NOTE: Further steps required to retrieve more than 100 guilds.
pub async fn fetch_all_active_guilds(ctx: &Context) -> Vec<GuildInfo> {
    ctx.http
        .get_guilds(None, Some(100))
        .await
        .expect("Could not fetch active guilds.")
}

/// NOTE: Further steps required to retrieve more than 1000 members.
pub async fn fetch_all_guild_members(ctx: &Context, guild_id: &GuildId) -> Vec<Member> {
    guild_id
        .members(&ctx.http, None, None)
        .await
        .expect("Could not fetch guild members.")
}
