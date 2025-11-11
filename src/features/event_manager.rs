//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]

use serenity::all::{Context, GuildId};
use sqlx::types::BigDecimal;

use crate::meetup::scrape::{self, get_meetup_group_data};

// set data to be refetched once every hour
const REFETCH_MEETUP_DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

pub fn run_scheduler(ctx: &Context, pool: &sqlx::PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REFETCH_MEETUP_DATA_INTERVAL);
        loop {
            interval.tick().await;
            // repeated task here...
        }
    });
}

async fn sync_meetup_discord_events(pool: &sqlx::PgPool) {
    let groups = sqlx::query!("SELECT group_name FROM meetup_groups")
        .fetch_all(pool)
        .await
        .unwrap();
    for group in groups {
        let group_data = scrape::get_meetup_group_data(&group.group_name);
        // for each discord group...
    }
}

/// given a guild id, fetch all (active) discord events in that guild and
/// attempt to populate the db, linking with meetup events as needed
/// NOTE: I maybe shouldn't have made this. Providing this function implies the intention of
/// recovering data after (partial or complete) sql data loss as opposed to a complete bot
/// reset.
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

        let event_id = BigDecimal::from(discord_event.id.get());
        let event_description = discord_event.description.as_ref().unwrap();

        // if event not in db, add to db
        sqlx::query!(
            r#"
            INSERT INTO discord_events (discord_event_id)
            VALUES ($1)
            ON CONFLICT DO NOTHING
            "#,
            event_id
        )
        .execute(pool)
        .await?;

        // populate linker table between discord events and meetup events
        let meetup_event_ids = get_meetup_events_from_discord_event(&event_description);
        for meetup_event_id in meetup_event_ids.iter() {
            sqlx::query!(
                r#"
                INSERT INTO discord_events_meetup_events (discord_event_id, meetup_event_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
                event_id,
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

// every hour:
//  - pull meetup data
//  - for each event:
//      - if event not in db:
//          - if event with duplicate hash exists in db:
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
    use serenity::all::Timestamp;

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
