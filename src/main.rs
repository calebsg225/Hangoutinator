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
        GuildMemberUpdateEvent, Member, OnlineStatus, Ready, RoleId, UserId,
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

        println!(
            "user is not verified: {}, user has the verified role: {}",
            is_unverified, user_has_role
        );

        if is_unverified && user_has_role {
            let _ = welcome_channel_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().content("[WELCOME MESSAGE HERE]"),
                )
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
    dotenv::dotenv().expect("Failed to load .env file.");

    let token = env::var("TOKEN").expect("Expected a token in the environment.");
    println!("{token}");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        // used to detect role assignment/reassignment
        | GatewayIntents::GUILD_MEMBERS;

    // build client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .status(OnlineStatus::Idle)
        .await
        .expect("Err creating client");

    {
        // add collections to client
        let mut data = client.data.write().await;
        data.insert::<UnverifiedMemberCollection>(HashMap::default());
    }

    // TODO: add all unverified members in [each] guild to `UnverifiedMemberCollection`

    // start a single shard, and start listening to events
    //
    // shards will automatically attempt to reconnect, and will preform exponential backoff until
    // it reconnects
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
