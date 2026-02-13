//! src/commands/owner/set_bot_access_role.rs

use poise::serenity_prelude as seren;
use seren::{Mentionable, Role};
use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::{Context, Error};

/// Users with this role will have access to 'admin' bot commands, ex. `/meetup`, `/set`.
///
/// TODO: make a help command to desegnate admin/regular bot commands
#[poise::command(slash_command, rename = "set_bot_access_role")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "The role to give command access to. Leave blank to remove stored role."] role: Option<Role>,
) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let Some(role) = role else {
        sqlx::query!(
            "UPDATE guilds SET access_role_id = NULL WHERE guild_id = $1",
            BigDecimal::from(ctx.guild_id().unwrap().get())
        )
        .execute(pool)
        .await?;
        util::send_reply(
            &ctx,
            true,
            &format!("No bot access role is set for this guild."),
        )
        .await?;
        return Ok(());
    };
    sqlx::query!(
        "UPDATE guilds SET access_role_id = $1 WHERE guild_id = $2",
        BigDecimal::from(role.id.get()),
        BigDecimal::from(ctx.guild_id().unwrap().get())
    )
    .execute(pool)
    .await?;

    let content = format!(
        "The new bot access role is {}. Any user who does not have this role or is not the server owner will not be able to use admin commands.",
        role.mention()
    );
    util::send_reply(&ctx, true, &content).await?;
    Ok(())
}
