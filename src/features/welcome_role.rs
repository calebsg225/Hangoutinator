//! src/features/welcome_role.rs

use std::collections::{HashMap, HashSet};

use serenity::{
    Client,
    all::{
        ChannelId, Context, CreateMessage, GuildId, GuildMemberUpdateEvent, Mentionable, RoleId,
        UserId,
    },
    prelude::TypeMapKey,
};

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
async fn remove_verified_member(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    let mut data = ctx.data.write().await;
    let unverified_members = data
        .get_mut::<UnverifiedMemberCollection>()
        .unwrap()
        .get_mut(&guild_id)
        .unwrap();
    unverified_members.remove(&user_id)
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
        remove_verified_member(&ctx, event.guild_id, event.user.id).await;
        let welcome_message = format!("Welcome, {}!", event.user.mention());
        let _ = welcome_channel_id
            .send_message(&ctx.http, CreateMessage::new().content(welcome_message))
            .await;
        println!("Member `{}` has been welcomed.", event.user.name);
    }
}

/// populates the `UnverifiedMemberCollection` collection on `client.data` with unverified members
/// from all active guilds
pub async fn populate_unverified_members(client: &Client, verified_role_id: &RoleId) {
    // fetch guilds the bot is in
    // NOTE: Further steps required to retrieve more than 100 guilds.
    let active_guilds = client.http.get_guilds(None, Some(100)).await.unwrap();

    {
        // in this scope: populate `UnverifiedMemberCollection` with unverified members from all
        // guilds
        let mut data = client.data.write().await;
        let global_unverified_members = data.get_mut::<UnverifiedMemberCollection>().unwrap();
        println!("Active guild count: {}", active_guilds.len());
        for guild in active_guilds.iter() {
            // fetch guild members for each guild the bot is in
            // NOTE: Further steps required to retrieve more than 1000 members.
            let guild_members = guild.id.members(&client.http, None, None).await.unwrap();
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
