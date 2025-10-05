//! src/meetup/scheduler.rs
#![allow(unused)]

use std::collections::HashMap;

use chrono::TimeDelta;
use serenity::all::{CreateScheduledEvent, ScheduledEvent, ScheduledEventType, Timestamp};

use crate::meetup::scrape::MeetupManager;

// TODO: For multi-guild support, use a db to store interval choices. As it stands, every guild
// will use these intervals.

/// the amount of time between meetup/discord event syncing
const UPDATE_ALL_INTERVAL: TimeDelta = TimeDelta::days(1);
/// the amount of time to wait after a meetup event is created before
/// adding the event to discord
const DELAY_POST_INTERVAL: TimeDelta = TimeDelta::hours(1);

/// syncs meetup events and discord events
pub struct DiscordEventScheduler {
    meetup_manager: MeetupManager,
    discord_events: Vec<ScheduledEvent>,
}

impl DiscordEventScheduler {
    pub fn new() -> DiscordEventScheduler {
        Self {
            meetup_manager: MeetupManager::new(),
            discord_events: Vec::new(),
        }
    }

    pub fn from(events: Vec<ScheduledEvent>) -> DiscordEventScheduler {
        Self {
            meetup_manager: MeetupManager::new(),
            discord_events: events,
        }
    }

    /// abstract away initial discord event creation
    /// TODO: create direct conversion from custom `Event` type?
    fn create_event_builder() {
        let base = CreateScheduledEvent::new(
            ScheduledEventType::External,
            "todo",
            Timestamp::parse("todo").unwrap(),
        );
        todo!();
    }

    // function to start the timer
    // inside the timer: every x hours:
    // - pull new meetup data
    // - check for meetup events not on discord
    // - if any, add new events to discord (or set to be added after 1 hr)
    // - update all other existing events with new data??

    fn delete_event() {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    #[test]
    fn convert_rfc3339_to_datetime_to_timestamp() {
        let rfc3339 = "2025-10-01T18:15:00-04:00";
        /// utc adjusted
        let utc_adjusted_rfc3339 = "2025-10-01T22:15:00.000Z";

        let datetime = DateTime::parse_from_rfc3339(rfc3339).unwrap().to_utc();
        let timestamp = Timestamp::from_unix_timestamp(datetime.timestamp()).unwrap();

        assert_eq!(timestamp.to_string(), utc_adjusted_rfc3339);
    }
}
