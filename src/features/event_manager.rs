//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]

use serenity::all::{
    Builder, Context, CreateScheduledEvent, GuildId, ScheduledEvent, ScheduledEventId,
    ScheduledEventType,
};
use sqlx::types::BigDecimal;

use crate::meetup::{
    model::JSONVenue,
    scrape::{self, MeetupEvent, get_meetup_group_data},
};

// set data to be refetched once every hour
const REFETCH_MEETUP_DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);

/// starts a background task for keeping discord events synced with
/// meetup events
pub fn run_scheduler(ctx: &Context, pool: &sqlx::PgPool) {
    let pool1 = pool.clone();
    let ctx1 = ctx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REFETCH_MEETUP_DATA_INTERVAL);
        loop {
            interval.tick().await;
            sync_meetup_discord_events(&ctx1, &pool1).await.unwrap();
        }
    });
}

/// pulls meetup events for all groups, updates discord events as needed
async fn sync_meetup_discord_events(
    ctx: &Context,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let groups = sqlx::query!("SELECT group_name FROM meetup_groups")
        .fetch_all(pool)
        .await?;
    println!("Syncing {} meetup groups...", groups.len());
    for group in groups {
        let tracking_guilds = sqlx::query!(
            "SELECT guild_id FROM meetup_groups_guilds WHERE group_name = $1",
            group.group_name
        )
        .fetch_all(pool)
        .await?;

        if tracking_guilds.len() == 0 {
            continue;
        }

        let group_data = scrape::get_meetup_group_data(&group.group_name).unwrap();
        let events: Vec<MeetupEvent> = group_data.get_events();
        println!(
            "Syncing {} events in meetup group `{}`...",
            events.len(),
            group.group_name
        );
        for meetup_event in events {
            let event = meetup_event.get_event();
            let event_group_id = event.group.strip_prefix("Group:").unwrap();
            let existing_event = sqlx::query!(
                "SELECT * FROM meetup_events WHERE meetup_event_id = $1",
                event.id
            )
            .fetch_optional(pool)
            .await?;
            match existing_event {
                Some(r) => {
                    // event exists: check hashes
                    // TODO: check for duplicates with hash
                }
                None => {
                    // create event hashes
                    // add meetup event to db
                    sqlx::query!(
                        "INSERT INTO meetup_events (meetup_event_id, meetup_group_id, event_hash, duplicate_event_hash, end_time) VALUES ($1, $2, $3, $4, $5)", 
                        event.id, 
                        BigDecimal::from(event_group_id.parse::<u64>().unwrap()),
                        "", // TODO: hash
                        "", // TODO: duplicate hash
                        event.endTime
                    )
                    .execute(pool)
                    .await?;

                    // add meetup event to all tracking discord guilds: save event id
                    let discord_event_builder = CreateScheduledEvent::from(&meetup_event);
                    for rec in tracking_guilds.iter() {
                        let discord_event = discord_event_builder
                            .clone()
                            .execute(
                                &ctx.http,
                                GuildId::from(rec.guild_id.to_string().parse::<u64>().unwrap()),
                            )
                            .await?;

                        // add discord event id to db
                        sqlx::query!(
                            "INSERT INTO discord_events (discord_event_id) VALUES ($1)",
                            BigDecimal::from(discord_event.id.get())
                        )
                        .execute(pool)
                        .await?;

                        // add linker between discord event and meetup event
                        sqlx::query!(
                        "INSERT INTO discord_events_meetup_events (discord_event_id, meetup_event_id) VALUES ($1, $2)",
                        BigDecimal::from(discord_event.id.get()),
                        event_group_id

                    )
                        .execute(pool)
                        .await?;
                    }
                }
            };
        }
        println!("Events synced.");
    }
    Ok(())
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

impl<'a> From<&MeetupEvent> for CreateScheduledEvent<'a> {
    fn from(v: &MeetupEvent) -> Self {
        let event = v.get_event();
        let ven = v.get_venue(&event.venue);
        CreateScheduledEvent::new(
            ScheduledEventType::External,
            event.title.to_string(),
            event.dateTime,
        )
        .description(event.description.to_string())
        .end_time(event.endTime)
        .location(match ven {
            Some(v) => v.location(),
            _ => String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use serenity::all::Timestamp;
    use sqlx::types::BigDecimal;

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

    #[test]
    fn u64_bigdecimal_conversions() {
        let num: u64 = 9824750932;
        let bd = BigDecimal::from(num);
        assert_eq!(num, bd.to_string().parse::<u64>().unwrap());
    }
}
