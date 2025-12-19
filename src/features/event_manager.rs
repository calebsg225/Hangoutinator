//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, FixedOffset, Local, TimeDelta};
use serenity::all::{
    Builder, Context, CreateScheduledEvent, EditScheduledEvent, GuildId, GuildInfo, ScheduledEvent,
    ScheduledEventId, ScheduledEventType,
};
use sqlx::types::BigDecimal;

use crate::Error;
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
        loop {
            interval.tick().await;
            if let Err(e) = sync_meetup_discord_events(&ctx1, &pool1).await {
                println!("Cound not sync events. Error: {}", e);
            }
        }
    });
}

async fn sync_meetup_discord_events(ctx: &Context, pool: &sqlx::PgPool) -> Result<(), Error> {
    let updates = populate_db_from_meetup_events(ctx, pool).await?;
    Ok(())
}

/// Fetches new meetup event data for all guild-tracked meetup groups, updates db with new data.
///
/// Returns:
/// - ids of existing discord events that need to be updated
/// - ids of new meetup events not linked to a discord event
async fn populate_db_from_meetup_events(
    ctx: &Context,
    pool: &sqlx::PgPool,
) -> Result<SyncUpdates, Error> {
    let mut res = SyncUpdates::new();
    // automatically filter out untracked meetup groups
    let meetup_groups = sqlx::query!(
        r#"
            SELECT DISTINCT mgg.group_name FROM meetup_groups_guilds AS mgg
            INNER JOIN guilds AS g
            ON g.guild_id = mgg.guild_id
        "#
    )
    .fetch_all(pool)
    .await?;
    println!("Syncing {} meetup groups...", meetup_groups.len());
    for group in meetup_groups {
        // fetch the guild ids of all guilds tracking this meetup group
        let guilds = sqlx::query!(
            "SELECT guild_id FROM meetup_groups_guilds WHERE group_name = $1",
            group.group_name
        )
        .fetch_all(pool)
        .await?;

        // scrape meetup site, aggregate into one struct
        let group_data = scrape::get_meetup_group_data(&group.group_name).unwrap();
        // all (immediate upcoming, up to 30) meetup events in this meetup group
        let events: Vec<MeetupEvent> = group_data.get_events();
        println!(
            "Syncing {} events in meetup group `{}`...",
            events.len(),
            group.group_name
        );
        for event in events {
            let event_id = event.id.clone();
            println!("Syncing event with id `{}`...", event_id);
            let event_hash = event.get_hash();
            let dup_hash = event.get_dup_hash();
            let existing_event = sqlx::query!(
                "SELECT * FROM meetup_events WHERE meetup_event_id = $1",
                event_id
            )
            .fetch_optional(pool)
            .await?;
            match existing_event {
                Some(r) => {
                    let outdated = resync_meetup_event(ctx, pool, r.event_hash, event).await?;
                    res.outdated_discord_events.extend(outdated);
                }
                None => {
                    let id = add_meetup_event(ctx, pool, event).await?;
                    res.orphan_meetup_events.insert(id);
                }
            };
            println!("Syncing process complete for event with id `{}`.", event_id);
        }
        println!(
            "Syncing complete for events in meetup group `{}`.",
            group.group_name
        );
    }
    println!("Syncing complete for all tracked meetup groups.");

    // update outdated discord event ids set after removing old meetup event data
    res.outdated_discord_events.extend(clean(ctx, pool).await?);
    Ok(res)
}

// Syncs a meetup event pulled from meetup.com with an existing meetup event
// stored in the db
//
// Returns a set of ids of discord events that need to be updated as a result of resyncing
async fn resync_meetup_event(
    ctx: &Context,
    pool: &sqlx::PgPool,
    old_hash: BigDecimal,
    new_event: MeetupEvent,
) -> Result<OutdatedDiscordEvents, Error> {
    let now = Local::now();
    let event_hash = new_event.get_hash();
    if old_hash == BigDecimal::from(event_hash) {
        sqlx::query!(
            "UPDATE meetup_events SET last_synced = $1 WHERE meetup_event_id = $2",
            now,
            new_event.id
        )
        .execute(pool)
        .await?;
        return Ok(HashSet::new());
    }
    // get affected discord events
    let outdated_discord_events = sqlx::query!(
        r#"
            SELECT DISTINCT deme.discord_event_id FROM discord_events_meetup_events AS deme
            INNER JOIN meetup_events AS me
            ON deme.meetup_event_id = me.meetup_event_id
            WHERE me.meetup_event_id = $1
        "#,
        new_event.id
    )
    .fetch_all(pool)
    .await?;

    // update existing event in db with new data
    sqlx::query!(
        r#"
            UPDATE meetup_events
            SET title = $1, 
                description = $2,
                event_hash = $3,
                duplicate_event_hash = $4,
                repeated_event_hash = $5,
                end_time = $6,
                last_synced = $7
            WHERE meetup_event_id = $8
        "#,
        new_event.title,
        new_event.description,
        BigDecimal::from(event_hash),
        BigDecimal::from(new_event.get_dup_hash()),
        BigDecimal::from(new_event.get_rep_hash()),
        new_event.end_time,
        now,
        new_event.id
    )
    .execute(pool)
    .await?;

    Ok(outdated_discord_events
        .iter()
        .map(|e| e.discord_event_id.to_owned())
        .collect())
}

// Adds a meetup event pulled from meetup.com to the db
async fn add_meetup_event(
    ctx: &Context,
    pool: &sqlx::PgPool,
    new_event: MeetupEvent,
) -> Result<BigDecimal, Error> {
    let now = Local::now();
    let event_id = BigDecimal::from(new_event.group.id.parse::<u64>().unwrap());
    sqlx::query!(
        r#"
            INSERT INTO meetup_events
            (
                meetup_event_id,
                meetup_group_id,
                title,
                description,
                event_hash,
                duplicate_event_hash,
                end_time,
                last_synced
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        new_event.id,
        // TODO: deal with cases where meetup event group id
        // is not an integer
        event_id,
        new_event.title,
        new_event.description,
        BigDecimal::from(new_event.get_hash()),
        BigDecimal::from(new_event.get_dup_hash()),
        new_event.end_time,
        now
    )
    .execute(pool)
    .await?;
    Ok(event_id)
}

/// removes meetup events from the db if:
///  - the event has expired
///  - the event was not synced during the most recent resync (indicating either a deleted meetup
///  event or an untracked meetup group)
///
/// This function is designed to be run directly after a sync has occured.
///
/// returns a set of all affected discord events to be updated later.
async fn clean(ctx: &Context, pool: &sqlx::PgPool) -> Result<OutdatedDiscordEvents, Error> {
    let now = Local::now();
    // select a time before the most recent sync but after the sync before that
    let outdated_last_synced = now
        .checked_sub_signed(TimeDelta::from_std(REFETCH_MEETUP_DATA_INTERVAL).unwrap())
        .unwrap();

    let discord_events_to_update = sqlx::query!(
        r#"
            SELECT DISTINCT deme.discord_event_id
            FROM discord_events_meetup_events AS deme
            INNER JOIN meetup_events AS me
            ON me.meetup_event_id = deme.meetup_event_id
            WHERE me.end_time < $1 OR me.last_synced < $2
        "#,
        now,
        outdated_last_synced
    )
    .fetch_all(pool)
    .await?;

    sqlx::query!(
        "DELETE FROM meetup_events WHERE end_time < $1 OR last_synced < $2",
        now,
        outdated_last_synced
    )
    .execute(pool)
    .await?;

    Ok(discord_events_to_update
        .iter()
        .map(|e| e.discord_event_id.to_owned())
        .collect::<HashSet<BigDecimal>>())
}

pub async fn populate_db_guilds(
    ctx: &Context,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let active_guilds = fetch_all_active_guilds(ctx).await;
    for guild in active_guilds {
        let guild_exists = sqlx::query!(
            "SELECT COUNT (guild_id) FROM guilds WHERE guild_id = $1",
            BigDecimal::from(guild.id.get())
        )
        .fetch_one(pool)
        .await?;

        if guild_exists.count.unwrap() == 0 {
            sqlx::query!(
                "INSERT INTO guilds (guild_id) VALUES ($1)",
                BigDecimal::from(guild.id.get())
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

/// WARN: DUPLICATE!!! put population functions together
///
/// fetch guilds the bot is in
/// NOTE: Further steps required to retrieve more than 100 guilds.
async fn fetch_all_active_guilds(ctx: &Context) -> Vec<GuildInfo> {
    ctx.http
        .get_guilds(None, Some(100))
        .await
        .expect("Could not fetch active guilds.")
}

type OutdatedDiscordEvents = HashSet<BigDecimal>;
type OrphanMeetupEvents = HashSet<BigDecimal>;

struct SyncUpdates {
    outdated_discord_events: OutdatedDiscordEvents,
    orphan_meetup_events: OrphanMeetupEvents,
}

impl SyncUpdates {
    fn new() -> Self {
        Self {
            outdated_discord_events: HashSet::new(),
            orphan_meetup_events: HashSet::new(),
        }
    }
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
