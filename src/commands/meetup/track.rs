//! src/commands/meetup/track.rs

use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "track")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "the name of the meetup group to track"] group_name: String,
) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let guild_id = BigDecimal::from(ctx.guild_id().unwrap().get());
    let tracked_group = sqlx::query!(
        "SELECT * FROM meetup_groups_guilds WHERE guild_id = $1 AND group_name = $2",
        guild_id,
        group_name
    )
    .fetch_optional(pool)
    .await?;

    if let Some(_) = tracked_group {
        util::send_reply(
            &ctx,
            true,
            &format!("Meetup group `{group_name}` is already being tracked!"),
        )
        .await?;
        return Ok(());
    }

    let meetup_group = sqlx::query!(
        "SELECT * FROM meetup_groups WHERE group_name = $1",
        group_name
    )
    .fetch_optional(pool)
    .await?;

    if let None = meetup_group {
        sqlx::query!(
            "INSERT INTO meetup_groups (group_name) VALUES ($1)",
            group_name
        )
        .execute(pool)
        .await?;
    }

    sqlx::query!(
        "INSERT INTO meetup_groups_guilds (guild_id, group_name) VALUES ($1, $2)",
        guild_id,
        group_name
    )
    .execute(pool)
    .await?;

    util::send_reply(
        &ctx,
        true,
        &format!("You are now tracking the meetup group `{group_name}`."),
    )
    .await?;
    Ok(())
}
