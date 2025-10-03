//! src/meetup/scheduler.rs
#![allow(unused)]

use chrono::TimeDelta;
use serenity::all::{CreateScheduledEvent, ScheduledEventType, Timestamp};

use crate::meetup::scrape::MeetupManager;

const ONE_HOUR: TimeDelta = TimeDelta::hours(1);
const ONE_DAY: TimeDelta = TimeDelta::days(1);

/// syncs meetup events and discord events
pub struct DiscordEventScheduler {
    meetup_manager: MeetupManager,
}

impl DiscordEventScheduler {
    pub fn new() -> DiscordEventScheduler {
        let scheduler = Self {
            meetup_manager: MeetupManager::new(),
        };
        scheduler
    }

    fn create_event_builder() {
        CreateScheduledEvent::new(
            ScheduledEventType::External,
            "todo",
            Timestamp::parse("todo").unwrap(),
        );
        todo!();
    }
    fn update_event_builder() {
        todo!();
    }
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
