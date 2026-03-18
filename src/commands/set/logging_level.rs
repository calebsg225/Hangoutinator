//! src/commands/set/logging_level.rs

use sqlx::types::BigDecimal;

use crate::features::logging;
use crate::{Context, Error, commands::_util as util};

/// Set the logging channel. If a logging_level is set, bot logs will be sent here.
#[poise::command(slash_command, rename = "logging_level")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "Logging Level"] level: logging::DiscordLogLevel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let pool = &ctx.data().pool;
    let level_num = level.clone() as i32;

    let guild_info = sqlx::query!(
        "SELECT logging_level FROM guilds WHERE guild_id = $1",
        BigDecimal::from(guild_id.get())
    )
    .fetch_one(pool)
    .await?;

    if let Some(old_level) = guild_info.logging_level
        && old_level == level_num
    {
        let content = format!("{:?} is already set as the logging level.", level);
        util::send_reply(&ctx, true, &content).await?;
        return Ok(());
    }

    sqlx::query!(
        "UPDATE guilds SET logging_level = $1 WHERE guild_id = $2",
        level_num,
        BigDecimal::from(guild_id.get())
    )
    .execute(pool)
    .await?;

    let content = format!("Logging level set to {:?}.", level);
    util::send_reply(&ctx, true, &content).await?;
    Ok(())
}
