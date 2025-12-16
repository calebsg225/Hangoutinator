//! src/commands/command_auth.rs

use serenity::all::{GuildId, RoleId};
use sqlx::types::BigDecimal;

use crate::{Context, Error};

/// check if either the user has an access role or is the guild owner
pub async fn has_access(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = ctx.guild_id().unwrap();
    let has_role = match get_access_role(&ctx.data().pool, guild_id).await {
        Some(role) => ctx.author().has_role(ctx, guild_id, role).await?,
        None => false,
    };
    let guild = ctx.http().get_guild(guild_id).await?;
    let is_guild_owner = guild.owner_id == ctx.author().id;
    Ok(has_role || is_guild_owner)
}

/// fetch the access role from the db
async fn get_access_role(pool: &sqlx::PgPool, guild_id: GuildId) -> Option<RoleId> {
    let access_role = sqlx::query!(
        r#"
            SELECT admin_role_id FROM guilds WHERE guild_id = $1
        "#,
        BigDecimal::from(guild_id.get())
    )
    .fetch_optional(pool)
    .await
    .unwrap_or_else(|_| {
        println!("Could not fetch role from db.");
        return None;
    });
    let record = access_role?;
    let role = record.admin_role_id?;
    let role = RoleId::from(role.to_string().parse::<u64>().unwrap());
    Some(role)
}
