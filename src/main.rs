//! main.rs

// TODO: improve file structure. Right now it's all in `main.rs`.

use std::{
    collections::{HashMap, HashSet},
    env,
};

use serenity::{
    Client,
    all::{
        ChannelId, Context, CreateMessage, EventHandler, GatewayIntents, GuildId,
        GuildMemberUpdateEvent, Member, Mentionable, OnlineStatus, Ready, RoleId, UserId,
    },
    async_trait,
    prelude::TypeMapKey,
};

// collection to keep track of members without the verified role
struct UnverifiedMemberCollection;

impl TypeMapKey for UnverifiedMemberCollection {
    type Value = HashMap<GuildId, HashSet<UserId>>;
}

/// Checks `UnverifiedMemberCollection` contains a specific user in a specific guild
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

/// Removes a verified user from `UnverifiedMembersCollection`
async fn remove_verified_member(ctx: &Context, guild_id: GuildId, user_id: UserId) -> bool {
    let mut data = ctx.data.write().await;
    let unverified_members = data
        .get_mut::<UnverifiedMemberCollection>()
        .unwrap()
        .get_mut(&guild_id)
        .unwrap();
    unverified_members.remove(&user_id)
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_update(
        &self,
        ctx: Context,
        _: Option<Member>,
        _: Option<Member>,
        event: GuildMemberUpdateEvent,
    ) {
        // get the role id and channel id from .env
        // NOTE: Storing the channel id and role id in .env will only work for a single guild.
        // TODO: multi-guild support (with a database)
        // TODO: deal with these fucking unwrap()s!!
        let verified_role_id: RoleId = RoleId::from(
            env::var("VERIFIED_ROLE_ID")
                .expect("Expected a verified role id in the environment.")
                .parse::<u64>()
                .unwrap(),
        );
        let welcome_channel_id: ChannelId = ChannelId::from(
            env::var("WELCOME_CHANNEL_ID")
                .expect("Expected a welcome channel id in the environment.")
                .parse::<u64>()
                .unwrap(),
        );

        println!("A guild member update has occured.");

        // check that the user is in `UnverifiedMemberCollection`.
        let is_unverified = is_unverified_member(&ctx, event.guild_id, event.user.id).await;
        // check that the user has the role.
        let user_has_role = event.roles.contains(&verified_role_id);

        if is_unverified && user_has_role {
            remove_verified_member(&ctx, event.guild_id, event.user.id).await;
            let welcome_message = format!("Welcome, {}!", event.user.mention());
            let _ = welcome_channel_id
                .send_message(&ctx.http, CreateMessage::new().content(welcome_message))
                .await;
        }
    }

    /// runs when the bot is ready
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // load env vars
    // This will not panic.
    dotenv::dotenv().unwrap_or_default();

    let token = env::var("TOKEN").expect("Expected a token in the environment.");

    // NOTE: Not compatable with more than a single guild.
    // TODO: Use a database.
    let verified_role_id: RoleId = RoleId::from(
        env::var("VERIFIED_ROLE_ID")
            .expect("Expected a verified role id in the environment.")
            .parse::<u64>()
            .unwrap(),
    );

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        // used to detect role assignment/reassignment
        | GatewayIntents::GUILD_MEMBERS;

    // build client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .status(OnlineStatus::Idle)
        .await
        .expect("Err creating client");

    {
        // in this scope: add collections to client
        let mut data = client.data.write().await;
        data.insert::<UnverifiedMemberCollection>(HashMap::default());
    }

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

    // start a single shard, and start listening to events
    //
    // shards will automatically attempt to reconnect, and will preform exponential backoff until
    // it reconnects
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
