//! src/commands/set/bot_access_role.rs

use poise::serenity_prelude as seren;
use seren::{Mentionable, Role};
use sqlx::types::BigDecimal;

use crate::commands::_helper as helper;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "bot_access_role")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "The role to give command access to"] role: Role,
) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    sqlx::query!(
        "UPDATE guilds SET access_role_id = $1 WHERE guild_id = $2",
        BigDecimal::from(role.id.get()),
        BigDecimal::from(ctx.guild_id().unwrap().get())
    )
    .execute(pool)
    .await?;

    let content = format!(
        "The new bot access role is {}. Any user who does not have this role or is not the server owner will not be able to use administration commands for this bot.",
        role.mention()
    );
    helper::send_reply(&ctx, true, &content).await?;
    Ok(())
}
