//! src/commands/meetup/track.rs

use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::features::event_manager;
use crate::{Context, Error};

const EPHEMERAL: bool = true;

/// Enter the URL name of a meetup group you want to be tracked in this server
#[poise::command(slash_command, rename = "track")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "URL meetup group name to track"] group_name: String,
) -> Result<(), Error> {
    let Some(guild_id) = &ctx.guild_id() else {
        util::send_reply(&ctx, EPHEMERAL, "You are not in a guild!").await?;
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let tracked_group = sqlx::query!(
        "SELECT * FROM meetup_groups_guilds WHERE group_name = $1 AND guild_id = $2",
        group_name,
        BigDecimal::from(guild_id.get())
    )
    .fetch_optional(pool)
    .await?;

    if let Some(_) = tracked_group {
        util::send_reply(
            &ctx,
            EPHEMERAL,
            &format!("Meetup group `{group_name}` is already being tracked!"),
        )
        .await?;
        return Ok(());
    }

    let group_exists = sqlx::query!(
        "SELECT DISTINCT * FROM meetup_groups WHERE group_name = $1",
        group_name
    )
    .fetch_optional(pool)
    .await?;

    if let None = group_exists {
        sqlx::query!(
            "INSERT INTO meetup_groups (group_name) VALUES ($1)",
            group_name
        )
        .execute(pool)
        .await?;
    }

    sqlx::query!(
        "INSERT INTO meetup_groups_guilds (group_name, guild_id) VALUES ($1, $2)",
        group_name,
        BigDecimal::from(guild_id.get())
    )
    .execute(pool)
    .await?;

    event_manager::toggle_group_update(&ctx.serenity_context(), guild_id, &group_name).await?;

    util::send_reply(
        &ctx,
        EPHEMERAL,
        &format!("You are now tracking the meetup group `{group_name}`.\n For changes to tracked meetup groups to come into effect, you may wait for the next resync, or force the issue with `/meetup resync`."),
    )
    .await?;
    Ok(())
}
