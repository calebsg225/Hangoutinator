//! src/commands/set/logging_channel.rs

use serenity::all::{Channel, Mentionable};
use sqlx::types::BigDecimal;

use crate::{Context, Error, commands::_util as util};

/// Set the logging channel. If a logging_level is set, bot logs will be sent here.
#[poise::command(slash_command, rename = "logging_channel")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "Logging Channel"] channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let pool = &ctx.data().pool;

    let guild_info = sqlx::query!(
        "SELECT logging_channel_id FROM guilds WHERE guild_id = $1",
        BigDecimal::from(guild_id.get())
    )
    .fetch_one(pool)
    .await?;

    if let Some(old_channel_id) = guild_info.logging_channel_id
        && old_channel_id == BigDecimal::from(channel.id().get())
    {
        let content = format!(
            "{} is already set as the logging channel.",
            channel.mention()
        );
        util::send_reply(&ctx, true, &content).await?;
        return Ok(());
    }

    sqlx::query!(
        "UPDATE guilds SET logging_channel_id = $1 WHERE guild_id = $2",
        BigDecimal::from(channel.id().get()),
        BigDecimal::from(guild_id.get())
    )
    .execute(pool)
    .await?;

    let content = format!("{} is now the logging channel.", channel.mention());
    util::send_reply(&ctx, true, &content).await?;
    Ok(())
}
