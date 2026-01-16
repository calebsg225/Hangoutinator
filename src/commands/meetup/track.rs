//! src/commands/meetup/track.rs

use crate::commands::_util as util;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "track")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "the name of the meetup group to track"] group_name: String,
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

    let meetup_group = sqlx::query!(
        "SELECT * FROM meetup_groups WHERE group_name = $1",
        group_name
    )
    .fetch_optional(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO meetup_groups (group_name) VALUES ($1)",
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
