//! src/commands/set/welcome_channel.rs

use serenity::all::{Channel, Mentionable};
use sqlx::types::BigDecimal;

use crate::{Context, Error, commands::_util as util};

#[poise::command(slash_command, rename = "welcome_channel")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "Welcome Channel"] channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let pool = &ctx.data().pool;

    let guild_info = sqlx::query!(
        "SELECT welcome_channel_id FROM guilds WHERE guild_id = $1",
        BigDecimal::from(guild_id.get())
    )
    .fetch_one(pool)
    .await?;

    if let Some(old_channel_id) = guild_info.welcome_channel_id
        && old_channel_id == BigDecimal::from(channel.id().get())
    {
        let content = format!(
            "{} is already set as the welcome channel.",
            channel.mention()
        );
        util::send_reply(&ctx, true, &content).await?;
        return Ok(());
    }

    sqlx::query!(
        "UPDATE guilds SET welcome_channel_id = $1 WHERE guild_id = $2",
        BigDecimal::from(channel.id().get()),
        BigDecimal::from(guild_id.get())
    )
    .execute(pool)
    .await?;

    let content = format!("{} is now the welcome channel.", channel.mention());
    util::send_reply(&ctx, true, &content).await?;
    Ok(())
}
