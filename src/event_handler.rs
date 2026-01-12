//! src/event_handler.rs

use serenity::{
    all::{
        ChannelId, Context, EventHandler, GuildId, GuildMemberUpdateEvent, Member, Ready, RoleId,
        User,
    },
    async_trait,
};
use sqlx::types::BigDecimal;

use crate::features::{self, event_manager};

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
        if let Some(wri) = guild_info.welcome_role_id
            && let Some(wci) = guild_info.welcome_channel_id
        {
            let _ = features::welcome_role::welcome_verified_member(
                &ctx,
                &event,
                &RoleId::from(wri.to_string().parse::<u64>().unwrap()),
                &ChannelId::from(wci.to_string().parse::<u64>().unwrap()),
            )
            .await;
        }
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
        println!("{} is connected!", ready.user.name);
        event_manager::populate_db_guilds(&ctx, &self.pool)
            .await
            .expect("Could not populate database with guilds.");
        features::welcome_role::populate_unverified_members(&ctx, &self.pool).await;
        //features::event_manager::run_scheduler(&ctx, &self.db_pool);
    }

    async fn shards_ready(&self, _ctx: Context, total_shards: u32) {
        println!("{} shard(s) ready", total_shards);
    }
}
