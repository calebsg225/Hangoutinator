//! src/commands/meetup/untrack.rs

use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "untrack")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "the name of the meetup group to stop tracking"] group_name: String,
) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let guild_id = BigDecimal::from(ctx.guild_id().unwrap().get());

    let tracked_group = sqlx::query!(
        "SELECT * FROM meetup_groups_guilds WHERE guild_id = $1 AND group_name = $2",
        guild_id,
        group_name,
    )
    .fetch_optional(pool)
    .await?;

    if let None = tracked_group {
        util::send_reply(
            &ctx,
            true,
            &format!("You are not currently tracking meetup group `{group_name}`."),
        )
        .await?;
        return Ok(());
    }

    let tracking_guilds = sqlx::query!(
        "SELECT * FROM meetup_groups_guilds WHERE guild_id != $1 AND group_name = $2",
        guild_id,
        group_name
    )
    .fetch_optional(pool)
    .await?;

    if let None = tracking_guilds {
        sqlx::query!(
            "DELETE FROM meetup_groups WHERE group_name = $1",
            group_name
        )
        .execute(pool)
        .await?;
    } else {
        sqlx::query!(
            "DELETE FROM meetup_groups_guilds WHERE guild_id = $1 AND group_name = $2",
            guild_id,
            group_name
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}
