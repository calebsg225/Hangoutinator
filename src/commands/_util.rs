//! src/commands/_util.rs

use poise::CreateReply;
use serenity::all::{GuildId, RoleId};
use sqlx::types::BigDecimal;

use crate::{Context, Error, IdExt};

/// check if the user either has an access role or is the guild owner
pub async fn has_access(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = ctx.guild_id().unwrap();
    let has_role = match get_access_role(&ctx.data().pool, guild_id).await {
        Some(role) => ctx.author().has_role(ctx, guild_id, role).await?,
        None => false,
    };
    let guild = ctx.http().get_guild(guild_id).await?;
    let is_guild_owner = guild.owner_id == ctx.author().id;
    let has_access = has_role || is_guild_owner;
    if !has_access {
        let content = "You do not have access to this command.";
        send_reply(&ctx, true, &content).await?;
    }
    Ok(has_access)
}

#[allow(unused)]
pub async fn is_guild_owner(ctx: Context<'_>) -> Result<bool, Error> {
    let guild = ctx.http().get_guild(ctx.guild_id().unwrap()).await?;
    let is_guild_owner = guild.owner_id == ctx.author().id;
    Ok(is_guild_owner)
}

/// fetch the access role from the db
async fn get_access_role(pool: &sqlx::PgPool, guild_id: GuildId) -> Option<RoleId> {
    let access_role = sqlx::query!(
        r#"
            SELECT access_role_id FROM guilds WHERE guild_id = $1
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
    let role = record.access_role_id?;
    let role = RoleId::from_big_decimal(&role).unwrap();
    Some(role)
}

/// `simple` way to send a reply to a command interaction
/// just another abstraction...
pub async fn send_reply(ctx: &Context<'_>, ephemeral: bool, content: &str) -> Result<(), Error> {
    let reply = CreateReply::default().ephemeral(ephemeral).content(content);
    ctx.send(reply).await?;
    Ok(())
}
