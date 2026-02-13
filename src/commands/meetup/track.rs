//! src/commands/meetup/track.rs

use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::features::event_manager;
use crate::{Context, Error};

const EPHEMERAL: bool = true;

/// Enter one or more URL names of meetup groups you want to be tracked in this server.
#[poise::command(slash_command, rename = "track")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "URL meetup group name(s) to track (whitespace separated)"] group_names: String,
) -> Result<(), Error> {
    let Some(guild_id) = &ctx.guild_id() else {
        util::send_reply(&ctx, EPHEMERAL, "You are not in a guild!").await?;
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let group_names = group_names.split_whitespace().collect::<Vec<&str>>();
    let mut newly_tracked_groups: Vec<&str> = Vec::new();
    let mut groups_already_tracked: Vec<&str> = Vec::new();
    for group_name in &group_names {
        let tracked_group = sqlx::query!(
            "SELECT * FROM meetup_groups_guilds WHERE group_name = $1 AND guild_id = $2",
            group_name,
            BigDecimal::from(guild_id.get())
        )
        .fetch_optional(pool)
        .await?;

        if let Some(_) = tracked_group {
            groups_already_tracked.push(group_name);
            continue;
        }

        newly_tracked_groups.push(group_name);
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
    }
    if groups_already_tracked.len() > 0 {
        util::send_reply(
            &ctx,
            EPHEMERAL,
            &format!(
                "You are already tracking the following meetup group(s):\n- `{}`",
                groups_already_tracked.join("`\n- `")
            ),
        )
        .await?;
    }
    if newly_tracked_groups.len() == 0 {
        util::send_reply(
            &ctx,
            EPHEMERAL,
            &format!("No untracked meetup groups; the group(s) entered are already being tracked."),
        )
        .await?;
        return Ok(());
    }
    util::send_reply(
            &ctx,
            EPHEMERAL,
            &format!("You are now tracking meetup group(s):\n- `{}`\n For changes to tracked meetup groups to come into effect, you may wait for the next refetch, or force the issue with `/meetup refetch`.", newly_tracked_groups.join("`\n- `")),
        )
        .await?;
    Ok(())
}
