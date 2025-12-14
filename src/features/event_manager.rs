//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]

use serenity::all::{
    Builder, Context, CreateScheduledEvent, EditScheduledEvent, GuildId, ScheduledEvent,
    ScheduledEventId, ScheduledEventType,
};
use sqlx::types::BigDecimal;

use crate::meetup::{
    model::MeetupEvent,
    scrape::{self, get_meetup_group_data},
};

// set data to be refetched once every hour
const REFETCH_MEETUP_DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);
//const REFETCH_MEETUP_DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// starts a background task for keeping discord events synced with
/// meetup events
pub fn run_scheduler(ctx: &Context, pool: &sqlx::PgPool) {
    let pool1 = pool.clone();
    let ctx1 = ctx.clone();
    println!("scheduler spawned");
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REFETCH_MEETUP_DATA_INTERVAL);
        let mut c = 0;
        loop {
            interval.tick().await;
            c += 1;
            println!("running : {}", c);
            sync_meetup_discord_events(&ctx1, &pool1).await.unwrap();
        }
    });
}

/// pulls meetup events for all groups, updates discord events as needed
async fn sync_meetup_discord_events(
    ctx: &Context,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let meetup_groups = sqlx::query!("SELECT group_name FROM meetup_groups")
        .fetch_all(pool)
        .await?;
    println!("Syncing {} meetup groups...", meetup_groups.len());
    for group in meetup_groups {
        // fetch the guild ids of all guilds tracking this meetup group
        let tracking_guilds = sqlx::query!(
            "SELECT guild_id FROM meetup_groups_guilds WHERE group_name = $1",
            group.group_name
        )
        .fetch_all(pool)
        .await?;

        // if the current meetup group is not tracked by any guilds, don't bother
        if tracking_guilds.len() == 0 {
            continue;
        }

        // scrape meetup site, aggregate into one struct
        let group_data = scrape::get_meetup_group_data(&group.group_name).unwrap();
        // all (immediate upcoming) meetup events in this meetup group
        let events: Vec<MeetupEvent> = group_data.get_events();
        println!(
            "Syncing {} events in meetup group `{}`...",
            events.len(),
            group.group_name
        );
        for event in events {
            let event_hash = event.generate_hash();
            let dup_hash = event.generate_dup_hash();
            let existing_event = sqlx::query!(
                "SELECT * FROM meetup_events WHERE meetup_event_id = $1",
                event.id
            )
            .fetch_optional(pool)
            .await?;
            match existing_event {
                Some(record) => {
                    // This meetup event is being tracked: check if it is up to date, resync
                    // if required.
                    if record.event_hash == BigDecimal::from(event_hash) {
                        continue;
                    }
                    resync_tracked_meetup_event(event, pool, ctx).await?;
                }
                None => {
                    // This meetup event is not being tracked: track it.
                    sync_untracked_meetup_event(
                        event,
                        tracking_guilds.iter().map(|r| r.guild_id.clone()).collect(),
                        pool,
                        ctx,
                    )
                    .await?;
                }
            };
        }
        println!("Events synced.");
    }
    Ok(())
}

// Updates an out-of-date tracked meetup event with fresh event data
// pulled from meetup.com.
async fn resync_tracked_meetup_event(
    new_event: MeetupEvent,
    pool: &sqlx::PgPool,
    ctx: &Context,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_hash = new_event.generate_hash();
    // get linked discord event
    let discord_events = sqlx::query!(
        r#"
            SELECT de.discord_event_id, de.guild_id
            FROM discord_events_meetup_events AS deme
            INNER JOIN discord_events AS de
            ON deme.discord_event_id = de.discord_event_id
            WHERE meetup_event_id = $1
        "#,
        new_event.id
    )
    .fetch_all(pool)
    .await?;
    // for each event, update
    for de in discord_events {
        let guild_id = GuildId::from(de.guild_id.to_string().parse::<u64>().unwrap());
        let event_id =
            ScheduledEventId::from(de.discord_event_id.to_string().parse::<u64>().unwrap());
        let edit_event_builder = EditScheduledEvent::from(&new_event);
        // edit discord event
        ctx.http
            .edit_scheduled_event(
                guild_id,
                event_id,
                &edit_event_builder,
                Some("sync with meetup.com event"),
            )
            .await?;
    }
    // update existing event in db with new data
    sqlx::query!(
        r#"
            UPDATE meetup_events
            SET event_hash = $1, duplicate_event_hash = $2, end_time = $3
            WHERE meetup_event_id = $4
        "#,
        BigDecimal::from(event_hash),
        BigDecimal::from(new_event.generate_dup_hash()),
        new_event.end_time,
        new_event.id
    )
    .execute(pool)
    .await?;
    Ok(())
    // TODO: check for duplicates with hash
}

// Starts tracking a previously untracked meetup event
async fn sync_untracked_meetup_event(
    new_event: MeetupEvent,
    tracking_guilds: Vec<BigDecimal>,
    pool: &sqlx::PgPool,
    ctx: &Context,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query!(
        "INSERT INTO meetup_events (meetup_event_id, meetup_group_id, event_hash, duplicate_event_hash, end_time) VALUES ($1, $2, $3, $4, $5)", 
        new_event.id,
        // TODO: deal with cases where meetup event group id
        // is not an integer
        BigDecimal::from(new_event.group.id.parse::<u64>().unwrap()),
        BigDecimal::from(new_event.generate_hash()),
        BigDecimal::from(new_event.generate_dup_hash()),
        new_event.end_time
    )
    .execute(pool)
    .await?;

    // add meetup event to all tracking discord guilds: save event id
    let discord_event_builder = CreateScheduledEvent::from(&new_event);
    for guild_id in tracking_guilds.iter() {
        let discord_event = discord_event_builder
            .clone()
            .execute(
                &ctx.http,
                GuildId::from(guild_id.to_string().parse::<u64>().unwrap()),
            )
            .await?;

        // add discord event id to db
        sqlx::query!(
            "INSERT INTO discord_events (discord_event_id, guild_id) VALUES ($1, $2)",
            BigDecimal::from(discord_event.id.get()),
            guild_id,
        )
        .execute(pool)
        .await?;

        // add linker between discord event and meetup event
        sqlx::query!(
            "INSERT INTO discord_events_meetup_events (discord_event_id, meetup_event_id) VALUES ($1, $2)",
            BigDecimal::from(discord_event.id.get()),
            new_event.id

        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

impl<'a> From<&MeetupEvent> for CreateScheduledEvent<'a> {
    fn from(event: &MeetupEvent) -> Self {
        CreateScheduledEvent::new(
            ScheduledEventType::External,
            event.title.to_string(),
            event.start_time,
        )
        .description(event.description.to_string())
        .end_time(event.end_time)
        .location(event.venue.location.to_string())
    }
}

impl<'a> From<&MeetupEvent> for EditScheduledEvent<'a> {
    fn from(event: &MeetupEvent) -> Self {
        EditScheduledEvent::new()
            .name(event.title.to_string())
            .start_time(event.start_time)
            .description(event.description.to_string())
            .end_time(event.end_time)
            .location(event.venue.location.to_string())
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use serenity::all::Timestamp;
    use sqlx::types::BigDecimal;
    use std::hash::{DefaultHasher, Hash, Hasher};

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

    /// mess around with big decimal
    #[test]
    fn u64_bigdecimal_conversions() {
        let num: u64 = 9824750932;
        let bd = BigDecimal::from(num);
        assert_eq!(num, bd.to_string().parse::<u64>().unwrap());
    }

    /// mess around with hashing
    #[test]
    fn hashing() {
        struct Person {
            id: u64,
            name: String,
            phone: u64,
            is_dup: bool,
        }

        impl Person {
            fn shared_hash<H: Hasher>(&self, state: &mut H) {
                self.id.hash(state);
                self.phone.hash(state);
            }
            fn def_hash(&self) -> u64 {
                let mut state = DefaultHasher::new();
                self.shared_hash(&mut state);
                state.finish()
            }
            fn dup_hash(&self) -> u64 {
                let mut state = DefaultHasher::new();
                self.shared_hash(&mut state);
                self.is_dup.hash(&mut state);
                state.finish()
            }
        }

        let p1 = Person {
            id: 5,
            name: "John".to_string(),
            phone: 555_666_7777,
            is_dup: false,
        };

        let p2 = Person {
            id: 5,
            name: "John".to_string(),
            phone: 555_666_7777,
            is_dup: true,
        };

        assert_ne!(p1.def_hash(), p1.dup_hash());
        assert_ne!(p2.def_hash(), p2.dup_hash());
        assert_eq!(p1.def_hash(), p2.def_hash());
        assert_ne!(p1.dup_hash(), p2.dup_hash());
    }
}
