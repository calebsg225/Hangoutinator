//! src/commands/meetup/list.rs

use sqlx::types::BigDecimal;
use std::fmt::Write;

use crate::commands::_util as util;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "list")]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let guild_id = ctx.guild_id().unwrap().get();
    let groups = sqlx::query!(
        r#"
            SELECT mgg.group_name
            FROM meetup_groups_guilds AS mgg
            INNER JOIN meetup_groups AS mg
            ON mg.group_name = mgg.group_name
            WHERE mgg.guild_id = $1
        "#,
        BigDecimal::from(guild_id)
    )
    .fetch_all(pool)
    .await?;
    if groups.len() == 0 {
        let content = "This guild is not tracking any meetup groups.";
        util::send_reply(&ctx, true, &content).await?;
    } else {
        let mut content = format!("This guild is tracking {} meetup groups:\n", groups.len());
        for group in groups {
            writeln!(content, " * {}", group.group_name).unwrap();
        }
        util::send_reply(&ctx, true, &content).await?;
    }
    Ok(())
}
