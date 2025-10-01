//! src/meetup/scheduler.rs
#![allow(unused)]

use std::time;

use crate::meetup::scrape::{MeetupManager, Populated};

const ONE_HOUR: time::Duration = time::Duration::new(3600, 0);
const ONE_DAY: time::Duration = time::Duration::new(86400, 0);

// NOTE: Discord Gateway Events:
// `Guild Scheduled Event Create`
// `Guild Scheduled Event Update`
// `Guild Scheduled Event Delete`
// `Guild Scheduled Event User Add`
// `Guild Scheduled Event User Remove`

/// syncs meetup events and discord events
pub struct DiscordEventScheduler {
    /// NOTE: An unpopulated `MeetupManager` isn't used...
    meetup_manager: MeetupManager<Populated>,
}

impl DiscordEventScheduler {
    pub fn new() -> DiscordEventScheduler {
        let scheduler = Self {
            meetup_manager: MeetupManager::new().populate(),
        };
        scheduler
    }
}
