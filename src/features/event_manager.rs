//! src/features/event_manager.rs

use std::collections::HashMap;

use serenity::{
    all::{Context, GuildId, ScheduledEvent, ScheduledEventStatus},
    prelude::TypeMapKey,
};

use crate::helper;
use crate::meetup::scheduler::DiscordEventScheduler;

/// A collection stored on the discord bot containing a `DiscordEventManager`
/// for each guild the bot is active in
pub struct DiscordEventSchedulerCollection;

impl TypeMapKey for DiscordEventSchedulerCollection {
    type Value = HashMap<GuildId, DiscordEventScheduler>;
}

/// populates `DiscordEventScheduler` with active events for each
/// guild the bot is active in
pub async fn populate_discord_events(ctx: &Context) {
    let active_guilds = helper::fetch_all_active_guilds(&ctx).await;

    let mut data = ctx.data.write().await;
    let event_scheduler_collection = data.get_mut::<DiscordEventSchedulerCollection>().unwrap();

    // NOTE: This loop exists twice atm, could be more in the future.
    // TODO: Combine the two loops.
    for guild in active_guilds.iter() {
        let events = helper::fetch_all_guild_events(&ctx, guild.id)
            .await
            .iter()
            .filter_map(|event| match event.status {
                ScheduledEventStatus::Scheduled => Some(event.to_owned()),
                ScheduledEventStatus::Active => Some(event.to_owned()),
                _ => None,
            })
            .collect::<Vec<ScheduledEvent>>();

        event_scheduler_collection.insert(guild.id, DiscordEventScheduler::from(events));
    }
}

/// Removes a stored discord event from a guilds collection
pub async fn remove_event_from_collection() {}
