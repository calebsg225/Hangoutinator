//! src/commands/set/welcome_role.rs

use serenity::all::{Mentionable, Role};
use sqlx::types::BigDecimal;

use crate::commands::_util as util;
use crate::features::welcome_role;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "welcome_member_role")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "Welcome Member Role"] role: Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let pool = &ctx.data().pool;

    let guild_info = sqlx::query!(
        "SELECT welcome_role_id FROM guilds WHERE guild_id = $1",
        BigDecimal::from(guild_id.get())
    )
    .fetch_one(pool)
    .await?;

    if let Some(old_role_id) = guild_info.welcome_role_id
        && old_role_id == BigDecimal::from(role.id.get())
    {
        let content = format!("{} is already set as the welcome role.", role.mention());
        util::send_reply(&ctx, true, &content).await?;
        return Ok(());
    }

    sqlx::query!(
        "UPDATE guilds SET welcome_role_id = $1 WHERE guild_id = $2",
        BigDecimal::from(role.id.get()),
        BigDecimal::from(guild_id.get())
    )
    .execute(pool)
    .await?;

    welcome_role::populate_unverified_guild_members(ctx.serenity_context(), guild_id, role.id)
        .await?;

    let content = format!("The welcome role has been set to {}.", role.mention());
    util::send_reply(&ctx, true, &content).await?;
    Ok(())
}
