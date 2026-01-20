//! src/commands/meetup/track.rs

use crate::commands::_util as util;
use crate::{Context, Error};

/// Enter the URL name of a meetup group you want me to start tracking.
#[poise::command(slash_command, rename = "track")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "URL meetup group name to track"] group_name: String,
) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let tracked_group = sqlx::query!(
        "SELECT * FROM meetup_groups WHERE group_name = $1",
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

    sqlx::query!(
        "INSERT INTO meetup_groups (group_name) VALUES ($1)",
        group_name
    )
    .execute(pool)
    .await?;

    util::send_reply(
        &ctx,
        true,
        &format!("You are now tracking the meetup group `{group_name}`.\n For changes to tracked meetup groups to come into effect, you may wait for the next resync, or force the issue with `/meetup resync`."),
    )
    .await?;
    Ok(())
}
