//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]
use std::collections::HashMap;

use chrono::TimeDelta;
use serenity::{
    all::{GuildId, Timestamp},
    prelude::TypeMapKey,
};

/// the amount of time between meetup/discord event syncing
const UPDATE_ALL_INTERVAL: TimeDelta = TimeDelta::days(1);
/// the amount of time to wait after a meetup event is created before
/// adding the event to discord
const DELAY_POST_INTERVAL: TimeDelta = TimeDelta::hours(1);

pub struct DiscordEventSchedulerCollection;

impl TypeMapKey for DiscordEventSchedulerCollection {
    type Value = HashMap<GuildId, DiscordEventScheduler>;
}

pub struct DiscordEventScheduler {}

// every hour:
//  - pull meetup data
//  - for each event:
//      - if event not in db: (also not in discord?)
//          - if event with duplicate id exists in db:
//              - update discord event
//          - else:
//              - add discord event (include meetup event ids for pairing)
//          - insert event into db
//          - STOP
//      - if event in db but hash != hash: (meetup event changed)
//          - update discord event
//          - replace stored hash in db (and other required info)
//          - STOP
//      - if event in db and hash == hash: do nothing

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    /// mess around with timestamps
    #[test]
    fn convert_rfc3339_to_datetime_to_timestamp() {
        let rfc3339 = "2025-10-01T18:15:00-04:00";
        // utc adjusted
        let utc_adjusted_rfc3339 = "2025-10-01T22:15:00.000Z";

        let datetime = DateTime::parse_from_rfc3339(rfc3339).unwrap().to_utc();
        let timestamp = Timestamp::from_unix_timestamp(datetime.timestamp()).unwrap();

        assert_eq!(timestamp.to_string(), utc_adjusted_rfc3339);
    }
}
