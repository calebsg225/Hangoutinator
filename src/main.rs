//! src/main.rs

use std::{collections::HashMap, env};

use serenity::{
    Client,
    all::{
        ChannelId, Context, EventHandler, GatewayIntents, GuildMemberUpdateEvent, Member,
        OnlineStatus, Ready, RoleId,
    },
    async_trait,
};

mod features;

use features::welcome_role;

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
        // NOTE: Storing the channel id and role id in .env will only work for a single guild.
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

        welcome_role::welcome_verified_member(&ctx, &event, &verified_role_id, &welcome_channel_id)
            .await;
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
    let verified_role_id: RoleId = RoleId::from(
        env::var("VERIFIED_ROLE_ID")
            .expect("Expected a verified role id in the environment.")
            .parse::<u64>()
            .unwrap(),
    );

    // `GUILD_MEMBERS` used to detect role assignment/reassignment
    let intents = GatewayIntents::GUILD_MEMBERS;

    // build client
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .status(OnlineStatus::Idle)
        .await
        .expect("Err creating client");

    {
        // in this scope: add collections to client
        let mut data = client.data.write().await;
        data.insert::<welcome_role::UnverifiedMemberCollection>(HashMap::default());
    }

    welcome_role::populate_unverified_members(&client, &verified_role_id).await;

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
