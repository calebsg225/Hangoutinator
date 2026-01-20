//! src/features/welcome_role.rs

use std::collections::{HashMap, HashSet};

use rand::seq::IndexedRandom;
use serenity::{
    all::{
        ChannelId, Context, CreateMessage, GuildId, GuildMemberUpdateEvent, Mention, Mentionable,
        RoleId, UserId,
    },
    prelude::TypeMapKey,
};

use crate::features::_util as util;
use crate::{Error, IdExt};

/// each message has 2 strings, one goes before the user mention, one goes after
const WELCOME_MESSAGES: [[&str; 2]; 3] = [
    ["Welcome, ", "!"],
    ["Glad you're here ", "."],
    ["Hello ", ", welcome aboard!"],
];

// collection to keep track of members without the verified role
pub struct UnverifiedMemberCollection;

impl TypeMapKey for UnverifiedMemberCollection {
    type Value = HashMap<GuildId, HashSet<UserId>>;
}

/// Checks if `UnverifiedMemberCollection` contains a specific member in a specific guild
async fn is_unverified_member(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<bool, Error> {
    let data = ctx.data.write().await;
    let unverified_members = data
        .get::<UnverifiedMemberCollection>()
        .unwrap()
        .get(&guild_id)
        .unwrap();
    Ok(unverified_members.contains(&user_id))
}

/// Removes a verified member from `UnverifiedMembersCollection`
pub async fn remove_member(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
) -> Result<bool, Error> {
    let mut data = ctx.data.write().await;
    let unverified_members = data
        .get_mut::<UnverifiedMemberCollection>()
        .unwrap()
        .get_mut(&guild_id)
        .unwrap();
    Ok(unverified_members.remove(&user_id))
}

pub async fn add_member(ctx: &Context, guild_id: GuildId, user_id: UserId) -> Result<bool, Error> {
    let mut data = ctx.data.write().await;
    let unverified_members = data
        .get_mut::<UnverifiedMemberCollection>()
        .unwrap()
        .get_mut(&guild_id)
        .unwrap();
    Ok(unverified_members.insert(user_id))
}

/// checks if a member has just been verified. If so, sends a welcome message.
pub async fn welcome_verified_member(
    ctx: &Context,
    event: &GuildMemberUpdateEvent,
    verified_role_id: &RoleId,
    welcome_channel_id: &ChannelId,
) -> Result<(), Error> {
    let is_unverified = is_unverified_member(&ctx, event.guild_id, event.user.id).await?;
    let member_has_role = event.roles.contains(&verified_role_id);

    if is_unverified && member_has_role {
        remove_member(&ctx, event.guild_id, event.user.id).await?;
        let welcome_message = build_welcome_message(event.user.mention());
        let _ = welcome_channel_id
            .send_message(&ctx.http, CreateMessage::new().content(welcome_message))
            .await;
        println!("Member `{}` has been welcomed.", event.user.name);
    }
    Ok(())
}

/// populates cache with unverified members from all guild
pub async fn populate_unverified_members(ctx: &Context, pool: &sqlx::PgPool) -> Result<(), Error> {
    let active_guilds = util::fetch_all_active_guilds(ctx).await;

    let guild_info = sqlx::query!("SELECT * FROM guilds").fetch_all(pool).await?;

    println!("Active guild count: {}", active_guilds.len());
    for guild in guild_info.iter() {
        if let Some(wri) = &guild.welcome_role_id {
            let verified_role_id = RoleId::from_big_decimal(wri)?;

            let guild_id = GuildId::from_big_decimal(&guild.guild_id)?;

            populate_unverified_guild_members(ctx, guild_id, verified_role_id).await?;
        }
    }
    Ok(())
}

/// populates cache with unverified members from a specific guild
pub async fn populate_unverified_guild_members(
    ctx: &Context,
    guild_id: GuildId,
    verified_role_id: RoleId,
) -> Result<(), Error> {
    let mut data = ctx.data.write().await;
    let global_unverified_members = data.get_mut::<UnverifiedMemberCollection>().unwrap();
    let guild_members = util::fetch_all_guild_members(&ctx, &guild_id).await;
    let unverified_guild_members: HashSet<UserId> = HashSet::from_iter(
        guild_members
            .iter()
            .filter(|m| !m.roles.contains(&verified_role_id))
            .map(|m| m.user.id),
    );
    println!(
        "Unverified member count in guild {}: {:?}",
        guild_id,
        unverified_guild_members.len()
    );
    global_unverified_members.insert(guild_id, unverified_guild_members);
    Ok(())
}

/// build a welcome message, choosing one at random
fn build_welcome_message(user_name: Mention) -> String {
    let [start, end] = WELCOME_MESSAGES.choose(&mut rand::rng()).unwrap();
    format!("{}{}{}", start, user_name, end)
}
