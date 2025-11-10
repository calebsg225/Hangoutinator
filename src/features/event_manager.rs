//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]
use std::collections::HashMap;

use chrono::TimeDelta;
use serenity::{
    all::{Context, GuildId, ScheduledEvent, Timestamp},
    prelude::TypeMapKey,
};
use sqlx::types::BigDecimal;

/// the amount of time between meetup/discord event syncing
const UPDATE_ALL_INTERVAL: TimeDelta = TimeDelta::days(1);
/// the amount of time to wait after a meetup event is created before
/// adding the event to discord
const DELAY_POST_INTERVAL: TimeDelta = TimeDelta::hours(1);

/// given a guild id, fetch all (active) discord events in that guild and
/// attempt to populate the db, linking with meetup events as needed
///
/// NOTE: This is to be run when the bot first starts
async fn populate_discord_events_into_db(
    ctx: &Context,
    pool: &sqlx::PgPool,
    guild_id: GuildId,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing_discord_events = ctx.http.get_scheduled_events(guild_id, false).await?;

    let bot_id = ctx.http.application_id().unwrap().get();
    for discord_event in existing_discord_events.iter() {
        // filter out events not created by the bot
        if discord_event.creator_id.unwrap().get() != bot_id {
            continue;
        }

        let event_id = discord_event.id.get();
        let event_description = discord_event.description.as_ref().unwrap();

        // if event not in db, add to db
        sqlx::query!(
            r#"
            INSERT INTO discord_events (discord_event_id)
            VALUES ($1)
            ON CONFLICT DO NOTHING
            "#,
            BigDecimal::from(&event_id)
        )
        .execute(pool)
        .await?;

        // populate linker table between discord events and meetup events
        let meetup_event_ids = get_meetup_events_from_discord_event(&event_description);
        for meetup_event_id in meetup_event_ids.iter() {
            let id: u128 = format!("${}${}", event_id, meetup_event_id)
                .parse()
                .unwrap();
            sqlx::query!(
                r#"
                INSERT INTO discord_events_meetup_events (id, discord_event_id, meetup_event_id)
                VALUES ($1, $2, $3)
                ON CONFLICT DO NOTHING
                "#,
                BigDecimal::from(id),
                BigDecimal::from(event_id),
                BigDecimal::from(meetup_event_id),
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

/// parses a bot-created discord events' description for all associated meetup event(s) id(s).
/// WARN: A predictable discord event description format must be used in order to successfully
/// retrieve the desired data.
fn get_meetup_events_from_discord_event(event_description: &str) -> Vec<u64> {
    // use a predetermined description format
    todo!();
}

// on startup:
//  - populate discord events
//  - if authored by bot and not in db:
//      - add to db
//      - pull meetup event ids and add links to db if not exists

// every hour:
//  - pull meetup data
//  - for each event:
//      - if event not in db:
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
