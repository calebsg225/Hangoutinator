//! src/event_handler.rs

use serenity::{
    all::{
        ChannelId, Context, EventHandler, GuildId, GuildMemberUpdateEvent, Member, Ready, RoleId,
        User,
    },
    async_trait,
};
use sqlx::types::BigDecimal;

use crate::{
    IdExt,
    features::{self, event_manager},
};

pub struct Handler {
    pub pool: sqlx::PgPool,
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
        let guild_info = sqlx::query!(
            "SELECT welcome_role_id, welcome_channel_id FROM guilds WHERE guild_id = $1",
            BigDecimal::from(event.guild_id.get())
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();

        let Some(welcome_role_id) = guild_info.welcome_role_id else {
            return;
        };
        let Some(welcome_channel_id) = guild_info.welcome_channel_id else {
            return;
        };

        let role_id = RoleId::from_big_decimal(&welcome_role_id).unwrap();
        let channel_id = ChannelId::from_big_decimal(&welcome_channel_id).unwrap();

        if let Err(e) =
            features::welcome_role::welcome_verified_member(&ctx, &event, &role_id, &channel_id)
                .await
        {
            println!("Could not welcome member. Error: {}", e);
        };
    }

    /// runs when a member joins a guild
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let user_id = new_member.user.id;

        if let Err(e) = features::welcome_role::add_member(&ctx, guild_id, user_id).await {
            println!("Could not add member to collection. Error: {}", e);
        };
    }

    /// runs when a member leaves/is removed from a guild
    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member_data: Option<Member>,
    ) {
        if let Err(e) = features::welcome_role::remove_member(&ctx, guild_id, user.id).await {
            println!("Could not remove member from collection. Error: {}", e);
        };
    }

    /// runs when the bot is ready
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        // NOTE: in this branch: only allows one guild
        event_manager::populate_db_guilds(&ctx, &self.pool)
            .await
            .expect("Could not populate database with guilds.");
        features::welcome_role::populate_unverified_members(&ctx, &self.pool)
            .await
            .expect("Could not populate cache with unverified members.");
        features::event_manager::run_scheduler(&ctx, &self.pool);
    }

    async fn shards_ready(&self, _ctx: Context, total_shards: u32) {
        println!("{} shard(s) ready", total_shards);
    }
}
