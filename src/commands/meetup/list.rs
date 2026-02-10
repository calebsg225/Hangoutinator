//! src/commands/meetup/list.rs

use std::fmt::Write;

use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::{Context, Error};

/// Display a list of all meetup groups currently being tracked in this server
///
/// TODO: make more pleasant to look at/read
/// TODO: Provide links???
#[poise::command(slash_command, rename = "list")]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = &ctx.guild_id() else {
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let groups = sqlx::query!(
        "SELECT DISTINCT group_name FROM meetup_groups_guilds WHERE guild_id = $1",
        BigDecimal::from(guild_id.get())
    )
    .fetch_all(pool)
    .await?;
    if groups.len() == 0 {
        let content = "This guild is not tracking any meetup groups.";
        util::send_reply(&ctx, true, &content).await?;
    } else {
        let mut content = format!(
            "This guild is tracking `{}` meetup group(s):\n",
            groups.len()
        );
        for group in groups {
            writeln!(content, " * `{}`", group.group_name).unwrap();
        }
        util::send_reply(&ctx, true, &content).await?;
    }
    Ok(())
}
