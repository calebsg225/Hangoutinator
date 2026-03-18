//! src/features/welcome_role.rs

use std::collections::{HashMap, HashSet};

use serenity::{
    all::{
        ChannelId, Context, CreateMessage, GuildId, GuildMemberUpdateEvent, Mentionable, RoleId,
        UserId,
    },
    prelude::TypeMapKey,
};
use sqlx::types::BigDecimal;

use crate::features::_util as util;
use crate::{Error, IdExt};

/// welcome messages to choose from at random that will be displayed after mentioning the user
const WELCOME_MESSAGES: [&str; 10] = [
    "Welcome to the side quest.",
    "Achievement unlocked: Socializing after 25.",
    "New member detected. Syncing sarcasm settings…",
    "Adulting optional. Memes mandatory.",
    "Where calendars are full but we still show up.",
    "This server runs on caffeine and mild existential humor.",
    "You’ve entered the “figuring it out” phase of life.",
    "This is your sign to mute your responsibilities for a bit.",
    "You made it. Vibes are strong, expectations are low.",
    "Welcome to the chaos respectfully organized.",
];

// collection to keep track of members without the verified role
pub struct UnverifiedMemberCollection;

impl TypeMapKey for UnverifiedMemberCollection {
    type Value = HashMap<GuildId, HashSet<UserId>>;
}

pub enum MemberAction {
    IsUnverified,
    Add,
    Remove,
}

/// Compares a user to data stored in cache, doing one of three actions:
///
/// - Checking whether the user has been verified: `MemberAction::IsUnverified`.
/// Will return true if the member is unverified.
/// - Adding a user to the `UnverifiedMemberCollection` in a guild
/// - Removing a user from the `UnverifiedMemberCollection` in a guild
pub async fn execute_member_action(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    action: MemberAction,
) -> Result<bool, Error> {
    let mut data = ctx.data.write().await;
    let col = data.get_mut::<UnverifiedMemberCollection>().unwrap();
    if !col.contains_key(&guild_id) {
        col.insert(guild_id.clone(), HashSet::new());
    }
    let unverified_members = col.get_mut(&guild_id).unwrap();
    let res = match action {
        MemberAction::IsUnverified => unverified_members.contains(&user_id),
        MemberAction::Add => unverified_members.insert(user_id),
        MemberAction::Remove => unverified_members.remove(&user_id),
    };
    Ok(res)
}

/// checks if a member has just been verified. If so, sends a welcome message.
pub async fn welcome_verified_member(
    ctx: &Context,
    pool: &sqlx::PgPool,
    event: &GuildMemberUpdateEvent,
) -> Result<(), Error> {
    let guild_info = sqlx::query!(
        "SELECT welcome_role_id, welcome_channel_id, message_index FROM guilds WHERE guild_id = $1",
        BigDecimal::from(event.guild_id.get())
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let Some(welcome_role_id) = guild_info.welcome_role_id else {
        return Ok(());
    };
    let Some(welcome_channel_id) = guild_info.welcome_channel_id else {
        return Ok(());
    };
    let mut message_index: usize = match guild_info.message_index {
        Some(i) => {
            let i = i as usize;
            if i > WELCOME_MESSAGES.len() - 1 { 0 } else { i }
        }
        None => 0,
    };

    let role_id = RoleId::from_big_decimal(&welcome_role_id).unwrap();
    let channel_id = ChannelId::from_big_decimal(&welcome_channel_id).unwrap();

    let is_unverified = execute_member_action(
        &ctx,
        event.guild_id,
        event.user.id,
        MemberAction::IsUnverified,
    )
    .await?;
    let member_has_role = event.roles.contains(&role_id);

    if is_unverified && member_has_role {
        execute_member_action(&ctx, event.guild_id, event.user.id, MemberAction::Remove).await?;
        let welcome_message = format!(
            "{} {}",
            event.user.mention(),
            WELCOME_MESSAGES[message_index]
        );
        message_index += 1;
        sqlx::query!(
            "UPDATE guilds SET message_index = $1 WHERE guild_id = $2",
            message_index as i32,
            BigDecimal::from(event.guild_id.get())
        )
        .execute(pool)
        .await?;
        let _ = channel_id
            .send_message(&ctx.http, CreateMessage::new().content(welcome_message))
            .await;
        println!("Member `{}` has been welcomed.", event.user.name);
    }
    Ok(())
}

/// populates cache with unverified members from all guilds
pub async fn populate_unverified_members(ctx: &Context, pool: &sqlx::PgPool) -> Result<(), Error> {
    let guild_info = sqlx::query!("SELECT * FROM guilds").fetch_all(pool).await?;

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
