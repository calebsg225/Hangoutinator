//! src/commands/meetup/untrack.rs

use crate::commands::_util as util;
use crate::{Context, Error};

/// Stop tracking a meetup group.
///
/// TODO: make it easier to stop tracking meetup groups (so you dont have
/// to type the whole name every time)
#[poise::command(slash_command, rename = "untrack")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "URL meetup group name to stop tracking"] group_name: String,
) -> Result<(), Error> {
    let pool = &ctx.data().pool;

    let tracked_group = sqlx::query!(
        "SELECT * FROM meetup_groups WHERE group_name = $1",
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

    sqlx::query!(
        "DELETE FROM meetup_groups WHERE group_name = $1",
        group_name
    )
    .execute(pool)
    .await?;

    util::send_reply(
        &ctx,
        true,
        &format!("You will no longer track meetup group `{group_name}`.\n For changes to tracked meetup groups to come into effect, you may wait for the next resync, or force the issue with `/meetup resync`."),
    )
    .await?;
    Ok(())
}
