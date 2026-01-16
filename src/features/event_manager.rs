//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly
#![allow(unused)]

use std::collections::HashSet;

use chrono::{DateTime, FixedOffset, Local, TimeDelta};
use serenity::all::{
    Context, CreateScheduledEvent, EditScheduledEvent, Guild, GuildId, ScheduledEventId,
    ScheduledEventType,
};
use sqlx::types::BigDecimal;

use crate::Error;
use crate::features::_util as util;
use crate::meetup::{
    model::MeetupEvent,
    scrape::{self, get_meetup_group_data},
};

// set data to be refetched once every hour
const REFETCH_MEETUP_DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);
const LAST_SYNCED_DELAY: std::time::Duration = std::time::Duration::from_secs(60);
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

pub async fn sync_meetup_discord_events(ctx: &Context, pool: &sqlx::PgPool) -> Result<(), Error> {
    match populate_db_from_meetup_events(ctx, pool).await {
        Ok(mut updates) => sync_discord_events(ctx, pool, &mut updates).await?,
        Err(e) => println!("{}", e),
    };
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
    // get all tracked meetup groups
    let meetup_groups = sqlx::query!("SELECT * FROM meetup_groups")
        .fetch_all(pool)
        .await?;
    if meetup_groups.len() == 0 {
        return Err(Error::from("No meetup groups are being tracked."));
    }
    println!("Syncing {} meetup groups...", meetup_groups.len());
    for group in meetup_groups {
        // scrape meetup site, aggregate into one struct
        let Ok(group_data) = scrape::get_meetup_group_data(&group.group_name) else {
            println!(
                "Failed to fetch meetup data for meetup group `{}`",
                group.group_name
            );
            continue;
        };
        // all (immediate upcoming, up to 30) meetup events in this meetup group
        let events: Vec<MeetupEvent> = group_data.get_events();
        println!(
            "Syncing {} events in meetup group `{}`...",
            events.len(),
            group.group_name
        );
        for event in events {
            let event_id = event.id.clone();
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
                    let outdated = update_meetup_event(ctx, pool, r.event_hash, event).await?;
                    res.outdated_discord_events.updated_me.extend(outdated);
                }
                None => {
                    if let Some(id) = add_meetup_event(ctx, pool, event).await? {
                        res.orphan_meetup_events.insert(id);
                    }
                }
            };
        }
        println!(
            "Syncing complete for events in meetup group `{}`.",
            group.group_name
        );
    }
    println!("Syncing complete for all tracked meetup groups.");

    // update outdated discord event ids set after removing old meetup event data
    // NOTE: this is shit...
    let updates = clean(ctx, pool).await?;
    res.outdated_discord_events
        .expired_me
        .extend(updates.expired_me);
    res.outdated_discord_events
        .removed_me
        .extend(updates.removed_me);
    Ok(res)
}

/// Uses the newly updated db to (re)sync discord events
///
/// Takes in:
/// - existing discord events that need updating
/// - newly created meetup events
async fn sync_discord_events(
    ctx: &Context,
    pool: &sqlx::PgPool,
    updates: &mut SyncUpdates,
) -> Result<(), Error> {
    let guild_info = sqlx::query!("SELECT * FROM guilds LIMIT 1")
        .fetch_one(pool)
        .await?;
    let guild_id = GuildId::from(guild_info.guild_id.to_string().parse::<u64>().unwrap());
    let mut meetup_ids_to_skip: HashSet<String> = HashSet::new();
    // deal with discord events affected by an expired meetup event
    for discord_event_id in &updates.outdated_discord_events.expired_me {
        // delete event and check for next repeated event
        let discord_event = sqlx::query_as!(
            DBDiscordEvent,
            "SELECT * FROM discord_events WHERE discord_event_id = $1",
            discord_event_id
        )
        .fetch_one(pool)
        .await?;

        sqlx::query!(
            "DELETE FROM discord_events WHERE discord_event_id = $1",
            discord_event_id
        )
        .execute(pool)
        .await?;

        create_next_rep_event(ctx, pool, discord_event).await?;
    }

    // deal with discord events affected by a removed meetup event
    for discord_event_id in &updates.outdated_discord_events.removed_me {
        // if there are existing duplicate events: keep event, update descr.
        // otherwise: delete event and check for next repeated event
        let discord_event = sqlx::query_as!(
            DBDiscordEvent,
            "SELECT * FROM discord_events WHERE discord_event_id = $1",
            discord_event_id
        )
        .fetch_one(pool)
        .await?;

        let scheduled_event_id =
            ScheduledEventId::from(discord_event_id.to_string().parse::<u64>().unwrap());

        let existing_duplicates = sqlx::query_as!(
            DBMeetupEvent,
            r#"
            SELECT me.*
            FROM meetup_events AS me
            INNER JOIN discord_events_meetup_events AS deme
            ON me.meetup_event_id = deme.meetup_event_id
            WHERE deme.discord_event_id = $1
            "#,
            discord_event_id
        )
        .fetch_all(pool)
        .await?;

        if existing_duplicates.len() == 0 {
            sqlx::query!(
                "DELETE FROM discord_events WHERE discord_event_id = $1",
                discord_event_id
            )
            .execute(pool)
            .await?;

            create_next_rep_event(ctx, pool, discord_event).await?;
        } else {
            manage_scheduled_event(
                ctx,
                ManageType::Edit(scheduled_event_id),
                existing_duplicates,
                guild_id,
                pool,
            )
            .await?;
        }
    }

    // deal with discord events affected by an updated meetup event
    for discord_event_id in &updates.outdated_discord_events.updated_me {
        let discord_event = sqlx::query_as!(
            DBDiscordEvent,
            "SELECT * FROM discord_events WHERE discord_event_id = $1",
            discord_event_id
        )
        .fetch_one(pool)
        .await?;

        let scheduled_event_id =
            ScheduledEventId::from(discord_event_id.to_string().parse::<u64>().unwrap());

        let existing_duplicates = sqlx::query_as!(
            DBMeetupEvent,
            r#"
            SELECT me.*
            FROM meetup_events AS me
            INNER JOIN discord_events_meetup_events AS deme
            ON me.meetup_event_id = deme.meetup_event_id
            WHERE deme.discord_event_id = $1
            "#,
            discord_event_id
        )
        .fetch_all(pool)
        .await?;

        if existing_duplicates.len() == 0 {
            sqlx::query!(
                "DELETE FROM discord_events WHERE discord_event_id = $1",
                discord_event_id
            )
            .execute(pool)
            .await?;

            manage_scheduled_event(
                ctx,
                ManageType::Delete(scheduled_event_id),
                existing_duplicates,
                guild_id,
                pool,
            )
            .await?;

            create_next_rep_event(ctx, pool, discord_event).await?;
        } else {
            manage_scheduled_event(
                ctx,
                ManageType::Edit(scheduled_event_id),
                existing_duplicates,
                guild_id,
                pool,
            )
            .await?;
        }
    }

    for meetup_event_id in &updates.orphan_meetup_events {
        if meetup_ids_to_skip.contains(meetup_event_id) {
            continue;
        }
        // these have no home: create events for all of them
        let meetup_event = sqlx::query_as!(
            DBMeetupEvent,
            "SELECT * FROM meetup_events WHERE meetup_event_id = $1",
            meetup_event_id
        )
        .fetch_one(pool)
        .await?;

        let related_events = sqlx::query_as!(
            DBMeetupEvent,
            "SELECT * FROM meetup_events WHERE repeated_event_hash = $1 ORDER BY start_time ASC",
            meetup_event.repeated_event_hash
        )
        .fetch_all(pool)
        .await?;

        meetup_ids_to_skip.extend(related_events.iter().map(|e| e.meetup_event_id.to_owned()));

        let main_event = &related_events[0];

        let upcoming_duplicates: Vec<DBMeetupEvent> = related_events
            .iter()
            .filter_map(|r| {
                if r.duplicate_event_hash == main_event.duplicate_event_hash {
                    return Some(r.to_owned());
                };
                None
            })
            .collect();

        let linked_discord_event = sqlx::query!(
            "SELECT COUNT(*) FROM discord_events WHERE duplicate_event_hash = $1",
            main_event.duplicate_event_hash
        )
        .fetch_one(pool)
        .await?;

        if let Some(count) = linked_discord_event.count
            && count == 0
        {
            manage_scheduled_event(ctx, ManageType::Create, upcoming_duplicates, guild_id, pool)
                .await?;
        }
    }
    Ok(())
}

/// check for the next upcoming repeated meetup event, creates a discord
/// event for it if required
/// NOTE: Requires group to be excluded from rep hash
async fn create_next_rep_event(
    ctx: &Context,
    pool: &sqlx::PgPool,
    discord_event: DBDiscordEvent,
) -> Result<(), Error> {
    let Some(rep_hash) = discord_event.repeated_event_hash else {
        return Ok(());
    };

    let guild_id = GuildId::from(discord_event.guild_id.to_string().parse::<u64>().unwrap());
    // NOTE: it would be useful here to ignore the group in rep hash
    let next_rep_event = sqlx::query!(
        "SELECT duplicate_event_hash FROM meetup_events WHERE repeated_event_hash = $1 ORDER BY start_time ASC LIMIT 1", 
        rep_hash
    ).fetch_optional(pool).await?;

    let Some(next_event) = next_rep_event else {
        return Ok(());
    };

    let existing_duplicates = sqlx::query_as!(
        DBMeetupEvent,
        "SELECT * FROM meetup_events WHERE duplicate_event_hash = $1",
        next_event.duplicate_event_hash
    )
    .fetch_all(pool)
    .await?;
    manage_scheduled_event(ctx, ManageType::Create, existing_duplicates, guild_id, pool).await?;
    Ok(())
}

enum ManageType {
    Create,
    Edit(ScheduledEventId),
    Delete(ScheduledEventId),
}

/// either create or edit a discord event, built from meetup event(s) data
/// stored in the db
///
/// creates/edits ONE DISCORD EVENT in ONE GUILD
///
/// all meetup events in `existing_duplicates` should only be from meetup
/// groups tracked by the guild
async fn manage_scheduled_event(
    ctx: &Context,
    manage_type: ManageType,
    existing_duplicates: Vec<DBMeetupEvent>,
    guild_id: GuildId,
    pool: &sqlx::PgPool,
) -> Result<(), Error> {
    if existing_duplicates.len() == 0 {
        return Ok(());
    };
    let mut description: Vec<String> = Vec::new();
    description.push("Meetup.com Event Link(s):\n".to_string());
    for event in &existing_duplicates {
        description.push(format!(
            "- https://meetup.com/{}/events/{}\n",
            event.meetup_group_name, event.meetup_event_id
        ));
    }

    let main_event = &existing_duplicates[0];

    description.push(format!("\n{}", main_event.description));

    match manage_type {
        ManageType::Create => {
            let new_discord_event = CreateScheduledEvent::new(
                ScheduledEventType::External,
                main_event.title.to_string(),
                main_event.start_time,
            )
            .description(description.join(""))
            .location(main_event.location.to_string())
            .end_time(main_event.end_time);

            let res = ctx
                .http
                .create_scheduled_event(guild_id, &new_discord_event, Some("New meetup.com data"))
                .await?;

            let new_scheduled_event_id = BigDecimal::from(res.id.get());

            sqlx::query!(
                r#"
                INSERT INTO discord_events (
                    discord_event_id, 
                    repeated_event_hash, 
                    duplicate_event_hash,
                    guild_id
                ) VALUES ($1, $2, $3, $4)
                "#,
                new_scheduled_event_id,
                existing_duplicates[0].repeated_event_hash,
                existing_duplicates[0].duplicate_event_hash,
                BigDecimal::from(guild_id.get())
            )
            .execute(pool)
            .await?;

            for dup in existing_duplicates {
                sqlx::query!(
                    r#"
                    INSERT INTO discord_events_meetup_events (
                        discord_event_id,
                        meetup_event_id
                    ) VALUES ($1, $2)
                    "#,
                    new_scheduled_event_id,
                    dup.meetup_event_id
                )
                .execute(pool)
                .await?;
            }
        }
        ManageType::Edit(scheduled_event_id) => {
            let edit_discord_event = EditScheduledEvent::new()
                .name(main_event.title.to_string())
                .description(description.join(""))
                .location(main_event.location.to_string())
                .start_time(main_event.start_time)
                .end_time(main_event.end_time);

            let _ = ctx
                .http
                .edit_scheduled_event(
                    guild_id,
                    scheduled_event_id,
                    &edit_discord_event,
                    Some("New meetup.com data"),
                )
                .await?;
        }
        ManageType::Delete(scheduled_event_id) => {
            let _ = ctx
                .http
                .delete_scheduled_event(guild_id, scheduled_event_id)
                .await?;
        }
    };
    Ok(())
}

// Syncs a meetup event pulled from meetup.com with an existing meetup event
// stored in the db
//
// Returns a set of ids of discord events that need to be updated as a result of resyncing
async fn update_meetup_event(
    ctx: &Context,
    pool: &sqlx::PgPool,
    old_hash: BigDecimal,
    new_event: MeetupEvent,
) -> Result<HashSet<BigDecimal>, Error> {
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
                location = $3,
                event_hash = $4,
                duplicate_event_hash = $5,
                repeated_event_hash = $6,
                start_time = $7,
                end_time = $8,
                last_synced = $9
            WHERE meetup_event_id = $10
        "#,
        new_event.title,
        new_event.description,
        new_event.venue.location,
        BigDecimal::from(event_hash),
        BigDecimal::from(new_event.get_dup_hash()),
        BigDecimal::from(new_event.get_rep_hash()),
        new_event.start_time,
        new_event.end_time,
        now,
        new_event.id
    )
    .execute(pool)
    .await?;

    // TODO: What to do if venue changes???

    Ok(outdated_discord_events
        .iter()
        .map(|e| e.discord_event_id.to_owned())
        .collect())
}

// Adds a meetup event pulled from meetup.com to the db
//
// Returns the id of the event if it will be an orphan.
async fn add_meetup_event(
    ctx: &Context,
    pool: &sqlx::PgPool,
    new_event: MeetupEvent,
) -> Result<Option<String>, Error> {
    let now = Local::now();
    let rep_hash = BigDecimal::from(new_event.get_rep_hash());
    sqlx::query!(
        r#"
            INSERT INTO meetup_events
            (
                meetup_event_id,
                meetup_group_id,
                meetup_group_name,
                title,
                description,
                location,
                event_hash,
                duplicate_event_hash,
                repeated_event_hash,
                start_time,
                end_time,
                last_synced
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        new_event.id,
        // TODO: deal with cases where meetup event group id
        // is not an integer
        BigDecimal::from(new_event.group.id.parse::<u64>().unwrap()),
        new_event.group.url_name,
        new_event.title,
        new_event.description,
        new_event.venue.location,
        BigDecimal::from(new_event.get_hash()),
        BigDecimal::from(new_event.get_dup_hash()),
        rep_hash,
        new_event.start_time,
        new_event.end_time,
        now
    )
    .execute(pool)
    .await?;
    Ok(Some(new_event.id))
}

/// Removes meetup events from the db if:
///  - the event has expired
///  - the event was not synced during the most recent resync (indicating either a deleted meetup
///  event or an untracked meetup group)
///
/// This function is designed to be run directly after a sync has occured.
///
/// returns a set of all affected discord events to be updated later.
async fn clean(ctx: &Context, pool: &sqlx::PgPool) -> Result<OutdatedDiscordEvents, Error> {
    let now = Local::now();
    let mut updates = OutdatedDiscordEvents::new();
    // select a time before the most recent sync but after the sync before that
    let outdated_last_synced = now
        .checked_sub_signed(TimeDelta::from_std(LAST_SYNCED_DELAY).unwrap())
        .unwrap();

    let expired_discord_events = sqlx::query!(
        r#"
            SELECT DISTINCT deme.discord_event_id
            FROM discord_events_meetup_events AS deme
            INNER JOIN meetup_events AS me
            ON me.meetup_event_id = deme.meetup_event_id
            WHERE me.end_time < $1
        "#,
        now,
    )
    .fetch_all(pool)
    .await?;

    let deleted_discord_events = sqlx::query!(
        r#"
            SELECT DISTINCT deme.discord_event_id
            FROM discord_events_meetup_events AS deme
            INNER JOIN meetup_events AS me
            ON me.meetup_event_id = deme.meetup_event_id
            WHERE me.last_synced < $1
            AND me.end_time > $2
        "#,
        outdated_last_synced,
        now
    )
    .fetch_all(pool)
    .await?;

    updates.expired_me.extend(
        expired_discord_events
            .iter()
            .map(|e| e.discord_event_id.to_owned())
            .collect::<HashSet<BigDecimal>>(),
    );

    updates.removed_me.extend(
        deleted_discord_events
            .iter()
            .map(|e| e.discord_event_id.to_owned())
            .collect::<HashSet<BigDecimal>>(),
    );

    sqlx::query!(
        "DELETE FROM meetup_events WHERE end_time < $1 OR last_synced < $2",
        now,
        outdated_last_synced
    )
    .execute(pool)
    .await?;

    Ok(updates)
}

pub async fn populate_db_guilds(
    ctx: &Context,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let active_guilds = util::fetch_all_active_guilds(ctx).await;
    let guilds = sqlx::query!("SELECT COUNT(*) FROM guilds")
        .fetch_one(pool)
        .await?;
    // if a guild is already in guilds, don't add a new one
    if let Some(count) = guilds.count
        && count > 0
    {
        return Ok(());
    };

    // the one guild
    let one_guild = &active_guilds[0];

    sqlx::query!(
        "INSERT INTO guilds (guild_id) VALUES ($1)",
        BigDecimal::from(one_guild.id.get())
    )
    .execute(pool)
    .await?;

    Ok(())
}

// NOTE: these structs are shit...
struct OutdatedDiscordEvents {
    expired_me: HashSet<BigDecimal>,
    removed_me: HashSet<BigDecimal>,
    updated_me: HashSet<BigDecimal>,
}

impl OutdatedDiscordEvents {
    fn new() -> Self {
        Self {
            expired_me: HashSet::new(),
            removed_me: HashSet::new(),
            updated_me: HashSet::new(),
        }
    }
}

/// Rust representation of sql `discord_events` table.
#[derive(Debug)]
struct DBDiscordEvent {
    discord_event_id: BigDecimal,
    repeated_event_hash: Option<BigDecimal>,
    duplicate_event_hash: Option<BigDecimal>,
    guild_id: BigDecimal,
}

/// Rust representation of sql `meetup_events` table.
#[derive(Debug, Clone)]
struct DBMeetupEvent {
    meetup_event_id: String,
    meetup_group_id: BigDecimal,
    meetup_group_name: String,
    title: String,
    description: String,
    location: String,
    event_hash: BigDecimal,
    duplicate_event_hash: BigDecimal,
    repeated_event_hash: BigDecimal,
    start_time: DateTime<FixedOffset>,
    end_time: DateTime<FixedOffset>,
    last_synced: DateTime<FixedOffset>,
}

type OrphanMeetupEvents = HashSet<String>;

struct SyncUpdates {
    outdated_discord_events: OutdatedDiscordEvents,
    orphan_meetup_events: OrphanMeetupEvents,
}

impl SyncUpdates {
    fn new() -> Self {
        Self {
            outdated_discord_events: OutdatedDiscordEvents::new(),
            orphan_meetup_events: HashSet::new(),
        }
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
