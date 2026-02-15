//! src/event_handler.rs

use serenity::{
    all::{
        ChannelId, Context, EventHandler, Guild, GuildId, GuildMemberUpdateEvent, Member, Ready,
        RoleId, UnavailableGuild, User,
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
        _old: Option<Member>, // cache feature
        _new: Option<Member>, // cache feature
        event: GuildMemberUpdateEvent,
    ) {
        // don't do anything if the updated user is this bot
        if ctx.cache.current_user().id == event.user.id {
            return;
        }

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
            println!("[ERROR] Could not welcome member. Error: {}", e);
        };
    }

    /// runs when a member joins a guild
    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let guild_id = new_member.guild_id;
        let user_id = new_member.user.id;

        if let Err(e) = features::welcome_role::execute_member_action(
            &ctx,
            guild_id,
            user_id,
            features::welcome_role::MemberAction::Add,
        )
        .await
        {
            println!("[ERROR] Could not add member to collection. Error: {}", e);
        };
    }

    /// runs when a member leaves/is removed from a guild
    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member_data: Option<Member>, // cache feature
    ) {
        // don't do anything if the user removed is this bot
        if ctx.cache.current_user().id == user.id {
            return;
        }

        if let Err(e) = features::welcome_role::execute_member_action(
            &ctx,
            guild_id,
            user.id,
            features::welcome_role::MemberAction::Remove,
        )
        .await
        {
            println!(
                "[ERROR] Could not remove member from collection. Error: {}",
                e
            );
        };
    }

    /// runs when the bot is added to a guild
    async fn guild_create(
        &self,
        _ctx: Context,
        guild: Guild,
        _is_new: Option<bool>, // cache feature
    ) {
        let id = guild.id.get();
        println!("[GUILD_CREATE] Adding guild with id [{}]...", id);
        let guild_id = BigDecimal::from(id);
        let was_added = event_manager::add_guild_to_db(&self.pool, guild_id)
            .await
            .expect("[ERROR] Failed when attempting to add guild to db.");
        println!("[GUILD_CREATE] Was added: [{}]", was_added);
    }

    /// runs when the bot is removed from a guild
    async fn guild_delete(
        &self,
        _ctx: Context,
        incomplete: UnavailableGuild,
        _full: Option<Guild>, // cache feature
    ) {
        // if `incomplete.unavailable` is false, bot was removed
        // TODO: clean up discord events and db?
        if !incomplete.unavailable {
            let id = incomplete.id.get();
            println!("[GUILD_DELETE] Removing guild with id [{}]...", id);
            let guild_id = BigDecimal::from(id);
            let was_removed = event_manager::remove_guild_from_db(&self.pool, guild_id)
                .await
                .expect("[ERROR] Failed when attempting to remove guild from db.");
            println!("[GUILD_DELETE] Was removed: [{}]", was_removed);
        }
    }

    /// runs when the bot is ready
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("[READY] {} is connected!", ready.user.name);
        event_manager::populate_db_guilds(&ctx, &self.pool)
            .await
            .expect("[ERROR] Could not populate database with guilds.");
        features::welcome_role::populate_unverified_members(&ctx, &self.pool)
            .await
            .expect("[ERROR] Could not populate cache with unverified members.");
        features::event_manager::run_scheduler(&ctx, &self.pool);
    }

    async fn shards_ready(&self, _ctx: Context, total_shards: u32) {
        println!("[READY] {} shard(s) ready", total_shards);
    }
}
