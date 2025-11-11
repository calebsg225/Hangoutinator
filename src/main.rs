//! src/main.rs

use std::{collections::HashMap, env};

use serenity::{
    Client,
    all::{
        ChannelId, Context, EventHandler, GatewayIntents, GuildId, GuildMemberUpdateEvent, Member,
        OnlineStatus, Ready, RoleId, User,
    },
    async_trait,
};

mod features;
mod helper;
mod meetup;

use features::event_manager;
use features::welcome_role;

struct Handler {
    db_pool: sqlx::PgPool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_update(
        &self,
        ctx: Context,
        _old: Option<Member>, // can't get cache to work...
        _new: Option<Member>, // can't get cache to work...
        event: GuildMemberUpdateEvent,
    ) {
        // TODO: store and pull from db
        let verified_role_id: RoleId = RoleId::from(
            env::var("VERIFIED_ROLE_ID")
                .expect("Expected a verified role id in the environment.")
                .parse::<u64>()
                .unwrap(),
        );
        // TODO: store and pull from db
        let welcome_channel_id: ChannelId = ChannelId::from(
            env::var("WELCOME_CHANNEL_ID")
                .expect("Expected a welcome channel id in the environment.")
                .parse::<u64>()
                .unwrap(),
        );

        println!("A guild member update has occured.");

        let _ = welcome_role::welcome_verified_member(
            &ctx,
            &event,
            &verified_role_id,
            &welcome_channel_id,
        )
        .await;
    }

    /// runs when a member joins a guild
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let user_id = new_member.user.id;

        println!(
            "Member with id {} has joined guild with id {}",
            &guild_id, &user_id
        );

        // when a member joins a guild, attempt to add them
        // to `UnverifiedMemberCollection`
        let _ = welcome_role::add_member(&ctx, guild_id, user_id).await;
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member_data: Option<Member>,
    ) {
        println!(
            "Member with id {} has been removed from guild with id {}.",
            &guild_id, &user.id
        );

        // when a member leaves a guild, attempt to remove them
        // from `UnverifiedMemberCollection`
        let _ = welcome_role::remove_member(&ctx, guild_id, user.id).await;
    }

    /// runs when the bot is ready
    async fn ready(&self, ctx: Context, ready: Ready) {
        // TODO: store and pull from db
        let verified_role_id: RoleId = RoleId::from(
            env::var("VERIFIED_ROLE_ID")
                .expect("Expected a verified role id in the environment.")
                .parse::<u64>()
                .unwrap(),
        );

        println!("{} is connected!", ready.user.name);
        welcome_role::populate_unverified_members(&ctx, &verified_role_id).await;
        event_manager::run_scheduler(&ctx, &self.db_pool);
    }

    async fn shards_ready(&self, _ctx: Context, total_shards: u32) {
        println!("{} shard(s) ready", total_shards);
    }
}

#[tokio::main]
async fn main() {
    // load env vars
    // This will not panic.
    dotenv::dotenv().unwrap_or_default();

    let token = env::var("TOKEN").expect("Expected a token in the environment.");

    // connect to database
    // TODO: pull db data from env?
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::postgres::PgConnectOptions::new()
                .host("database")
                .username("postgres")
                .password("foobar")
                .port(5432),
        )
        .await
        .expect("Could not connect to database.");

    // `GUILD_MEMBERS` used to detect role assignment/reassignment
    let intents = GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES;

    let handler = Handler { db_pool: pool };

    // build client
    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .status(OnlineStatus::Idle)
        .type_map_insert::<welcome_role::UnverifiedMemberCollection>(HashMap::default())
        .await
        .expect("Could not create client");

    if let Err(why) = client.start_autosharded().await {
        println!("Client error: {why:?}");
    }
}
