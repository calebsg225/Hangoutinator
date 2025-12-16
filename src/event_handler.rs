//! src/event_handler.rs

use std::env;

use serenity::{
    all::{
        ChannelId, Context, EventHandler, GuildId, GuildMemberUpdateEvent, Member, Ready, RoleId,
        User,
    },
    async_trait,
};

use crate::features;

pub struct Handler {
    pub db_pool: sqlx::PgPool,
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

        let _ = features::welcome_role::welcome_verified_member(
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
        let _ = features::welcome_role::add_member(&ctx, guild_id, user_id).await;
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
        let _ = features::welcome_role::remove_member(&ctx, guild_id, user.id).await;
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
        features::welcome_role::populate_unverified_members(&ctx, &verified_role_id).await;
        //features::event_manager::run_scheduler(&ctx, &self.db_pool);
    }

    async fn shards_ready(&self, _ctx: Context, total_shards: u32) {
        println!("{} shard(s) ready", total_shards);
    }
}
