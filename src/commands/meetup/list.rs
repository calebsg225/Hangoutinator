//! src/commands/meetup/list.rs

use std::fmt::Write;

use crate::commands::_util as util;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "list")]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let groups = sqlx::query!("SELECT * FROM meetup_groups")
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
