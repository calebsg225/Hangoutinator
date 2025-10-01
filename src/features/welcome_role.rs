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
async fn is_unverified_member(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    let data = ctx.data.write().await;
    // TODO: deal with these fucking unwrap()s!!
    let unverified_members = data
        .get::<UnverifiedMemberCollection>()
        .unwrap()
        .get(&guild_id)
        .unwrap();
    return unverified_members.contains(&user_id);
}

/// Removes a verified member from `UnverifiedMembersCollection`
pub async fn remove_member(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    let mut data = ctx.data.write().await;
    let unverified_members = data
        .get_mut::<UnverifiedMemberCollection>()
        .unwrap()
        .get_mut(&guild_id)
        .unwrap();
    unverified_members.remove(&user_id)
}

pub async fn add_member(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    let mut data = ctx.data.write().await;
    let unverified_members = data
        .get_mut::<UnverifiedMemberCollection>()
        .unwrap()
        .get_mut(&guild_id)
        .unwrap();
    unverified_members.insert(user_id)
}

/// checks if a member has been verified. If so, sends a welcome message.
pub async fn welcome_verified_member(
    ctx: &Context,
    event: &GuildMemberUpdateEvent,
    verified_role_id: &RoleId,
    welcome_channel_id: &ChannelId,
) {
    // check that the member is in `UnverifiedMemberCollection`.
    let is_unverified = is_unverified_member(&ctx, event.guild_id, event.user.id).await;
    // check that the member has the role.
    let member_has_role = event.roles.contains(&verified_role_id);

    if is_unverified && member_has_role {
        remove_member(&ctx, event.guild_id, event.user.id).await;
        let welcome_message = build_welcome_message(event.user.mention());
        let _ = welcome_channel_id
            .send_message(&ctx.http, CreateMessage::new().content(welcome_message))
            .await;
        println!("Member `{}` has been welcomed.", event.user.name);
    }
}

/// populates the `UnverifiedMemberCollection` collection on `client.data` with unverified members
/// from all active guilds
pub async fn populate_unverified_members(ctx: &Context, verified_role_id: &RoleId) {
    // fetch guilds the bot is in
    // NOTE: Further steps required to retrieve more than 100 guilds.
    let active_guilds = ctx.http.get_guilds(None, Some(100)).await.unwrap();

    {
        // in this scope: populate `UnverifiedMemberCollection` with unverified members from all
        // guilds
        let mut data = ctx.data.write().await;
        let global_unverified_members = data.get_mut::<UnverifiedMemberCollection>().unwrap();
        println!("Active guild count: {}", active_guilds.len());
        for guild in active_guilds.iter() {
            // fetch guild members for each guild the bot is in
            // NOTE: Further steps required to retrieve more than 1000 members.
            let guild_members = guild.id.members(&ctx.http, None, None).await.unwrap();
            let unverified_guild_members: HashSet<UserId> = HashSet::from_iter(
                guild_members
                    .iter()
                    .filter(|m| !m.roles.contains(&verified_role_id))
                    .map(|m| m.user.id),
            );
            println!(
                "Unverified member count in guild {}: {:?}",
                guild.id,
                unverified_guild_members.len()
            );
            global_unverified_members.insert(guild.id, unverified_guild_members);
        }
    }
}

/// build a welcome message, choosing one at random
fn build_welcome_message(user_name: Mention) -> String {
    let [start, end] = WELCOME_MESSAGES.choose(&mut rand::rng()).unwrap();
    format!("{}{}{}", start, user_name, end)
}
