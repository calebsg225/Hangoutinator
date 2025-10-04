//! src/features/event_manager.rs

use std::collections::HashMap;

use serenity::{all::GuildId, prelude::TypeMapKey};

use crate::meetup::scheduler::DiscordEventScheduler;

/// A collection stored on the discord bot containing a `DiscordEventManager`
/// for each guild the bot is active in
pub struct DiscordEventSchedulerCollection;

impl TypeMapKey for DiscordEventSchedulerCollection {
    type Value = HashMap<GuildId, DiscordEventScheduler>;
}

/// populates `DiscordEventScheduler` with active events for each
/// guild the bot is active in
pub async fn populate_discord_events() {}
