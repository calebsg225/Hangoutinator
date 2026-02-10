//! src/commands/meetup/untrack.rs

use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::event_manager;
use crate::{Context, Error};

const EPHEMERAL: bool = true;

/// Stop tracking a meetup group.
///
/// TODO: make it easier to stop tracking meetup groups (so you dont have
/// to type the whole name every time)
#[poise::command(slash_command, rename = "untrack")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "URL meetup group name to stop tracking"] group_name: String,
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

    if let None = tracked_group {
        util::send_reply(
            &ctx,
            EPHEMERAL,
            &format!("You are not currently tracking meetup group `{group_name}`."),
        )
        .await?;
        return Ok(());
    }

    let guilds_tracking = sqlx::query!(
        "SELECT * FROM meetup_groups_guilds WHERE group_name = $1 LIMIT 2",
        group_name
    )
    .fetch_all(pool)
    .await?;

    if guilds_tracking.len() == 1 {
        sqlx::query!(
            "DELETE FROM meetup_groups WHERE group_name = $1",
            group_name,
        )
        .execute(pool)
        .await?;
    } else {
        sqlx::query!(
            "DELETE FROM meetup_groups_guilds WHERE group_name = $1 AND guild_id = $2",
            group_name,
            BigDecimal::from(guild_id.get())
        )
        .execute(pool)
        .await?;
    }

    event_manager::toggle_group_update(&ctx.serenity_context(), guild_id, &group_name).await?;

    util::send_reply(
        &ctx,
        EPHEMERAL,
        &format!("You will no longer track meetup group `{group_name}`.\n For changes to tracked meetup groups to come into effect, you may wait for the next resync, or force the issue with `/meetup resync`."),
    )
    .await?;
    Ok(())
}
