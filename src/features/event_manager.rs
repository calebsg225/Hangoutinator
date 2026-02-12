//! src/features/event_manager.rs
//! periodically pull meetup events and update discord events accordingly

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, FixedOffset, Local, TimeDelta};
use serenity::all::{
    Context, CreateScheduledEvent, EditScheduledEvent, GuildId, ScheduledEventId,
    ScheduledEventType,
};
use serenity::prelude::TypeMapKey;
use sqlx::types::BigDecimal;

use crate::features::_util as util;
use crate::meetup::{
    model::MeetupEvent,
    scrape::{self},
};
use crate::{Error, IdExt};

const REFETCH_MEETUP_DATA_INTERVAL: std::time::Duration = std::time::Duration::from_secs(3600);
const LAST_SYNCED_DELAY: std::time::Duration = std::time::Duration::from_secs(60);

pub struct GroupUpdatesCollection;

impl TypeMapKey for GroupUpdatesCollection {
    type Value = HashMap<GuildId, HashSet<String>>;
}

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
    println!("[{}]", Local::now());
    match sync_meetup_events(pool).await {
        Ok(updates) => {
            sync_discord_events(ctx, pool, updates.to_owned()).await?;
        }
        Err(e) => println!("Failed to sync with meetup.com data. Error: {}", e),
    };
    Ok(())
}

/// Fetches new meetup event data for all guild-tracked meetup groups, updates db with new data.
async fn sync_meetup_events(pool: &sqlx::PgPool) -> Result<HashSet<BigDecimal>, Error> {
    // use the same time for all `last_synced` fields
    let now = Local::now();
    let mut res: HashSet<BigDecimal> = HashSet::new();
    // get all tracked meetup groups
    let meetup_groups = sqlx::query!("SELECT DISTINCT group_name FROM meetup_groups_guilds")
        .fetch_all(pool)
        .await?;
    if meetup_groups.len() == 0 {
        println!("No meetup groups are being tracked.");
    } else {
        println!("Fetching from [{}] meetup groups...", meetup_groups.len());
    }
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
        println!("Populating [{}] events...", events.len(),);
        for event in events {
            let event_id = event.id.clone();
            let existing_event = sqlx::query_as!(
                DBMeetupEvent,
                "SELECT * FROM meetup_events WHERE meetup_event_id = $1",
                event_id
            )
            .fetch_optional(pool)
            .await?;
            match existing_event {
                Some(e) => {
                    res.extend(update_meetup_event(pool, now, e, event).await?);
                }
                None => {
                    res.insert(add_meetup_event(pool, now, event).await?);
                }
            };
        }
        println!("Population complete.");
    }
    println!("Fetching complete for all tracked meetup groups.");

    res.extend(clean(pool, now).await?);
    Ok(res)
}

/// Uses the newly updated db to (re)sync discord events
///
/// takes in a set of all unique `collection_hash`s that need updating
async fn sync_discord_events(
    ctx: &Context,
    pool: &sqlx::PgPool,
    updates: HashSet<BigDecimal>,
) -> Result<(), Error> {
    // fetch only guilds that are tracking at least one meetup group
    let guilds_info = sqlx::query!("SELECT DISTINCT guild_id FROM meetup_groups_guilds")
        .fetch_all(pool)
        .await?;

    for guild in guilds_info {
        let guild_id = GuildId::from_big_decimal(&guild.guild_id)?;
        sync_guild_events(ctx, pool, &updates, guild_id, false).await?;
    }

    Ok(())
}

pub async fn get_all_guild_collection_hashes(
    pool: &sqlx::PgPool,
    guild_id: &GuildId,
) -> Result<HashSet<BigDecimal>, Error> {
    let hashes = sqlx::query!(
        r#"
            SELECT DISTINCT me.weekly_collection_hash
            FROM meetup_events AS me
            INNER JOIN meetup_groups_guilds AS mgg
            ON me.meetup_group_name = mgg.group_name
            WHERE mgg.guild_id = $1
        "#,
        BigDecimal::from(guild_id.get())
    )
    .fetch_all(pool)
    .await?;

    Ok(hashes
        .iter()
        .map(|h| h.weekly_collection_hash.to_owned())
        .collect())
}

pub async fn sync_guild_events(
    ctx: &Context,
    pool: &sqlx::PgPool,
    updates: &HashSet<BigDecimal>,
    guild_id: GuildId,
    preserve_tracking_changes: bool,
) -> Result<(), Error> {
    let mut update_count = 0;
    let mut create_count = 0;
    let mut delete_count = 0;
    println!("Syncing events in guild with id [{}]...", guild_id.get());

    // combine global updates with guild-specific tracking updates
    let updates = merge_tracking_updates(ctx, pool, &guild_id, updates).await?;

    if !preserve_tracking_changes {
        clear_group_updates(ctx, &guild_id).await?;
    }
    for collection_hash in &updates {
        let linked_discord_event = sqlx::query_as!(
            DBDiscordEvent,
            "SELECT * FROM discord_events WHERE collection_hash = $1 AND guild_id = $2",
            collection_hash,
            BigDecimal::from(guild_id.get()),
        )
        .fetch_optional(pool)
        .await?;

        // fetch the dup hash of the most recent meetup event where:
        // - the collection hash matches
        // - the meetup group the event belongs to is being tracked by the gulid
        let first_linked_meetup_event = sqlx::query!(
            r#"
                SELECT me.duplicate_event_hash
                FROM meetup_events AS me
                INNER JOIN meetup_groups_guilds AS mgg
                ON me.meetup_group_name = mgg.group_name
                WHERE me.weekly_collection_hash = $1
                AND mgg.guild_id = $2
                ORDER BY me.start_time ASC
                LIMIT 1
            "#,
            collection_hash,
            BigDecimal::from(guild_id.get())
        )
        .fetch_optional(pool)
        .await?;

        let Some(discord_event) = linked_discord_event else {
            // no discord event tied to collection hash: create event (if required)

            let Some(first_linked_meetup_event) = first_linked_meetup_event else {
                continue;
            };

            // fetch all duplicate meetup events where:
            // - the dup hash matches
            // - the meetup group the events belong to are being tracked by the guild
            let existing_duplicates = sqlx::query_as!(
                DBMeetupEvent,
                r#"
                    SELECT me.*
                    FROM meetup_events AS me
                    INNER JOIN meetup_groups_guilds AS mgg
                    ON me.meetup_group_name = mgg.group_name
                    WHERE me.duplicate_event_hash = $1
                    AND mgg.guild_id = $2
                    ORDER BY me.start_time ASC, me.meetup_group_name ASC
                "#,
                first_linked_meetup_event.duplicate_event_hash,
                BigDecimal::from(guild_id.get())
            )
            .fetch_all(pool)
            .await?;

            manage_scheduled_event(
                &ctx,
                ManageType::Create,
                existing_duplicates,
                guild_id,
                pool,
            )
            .await?;
            create_count += 1;
            continue;
        };
        // discord event already exists tied to collection hash.

        let scheduled_event_id =
            ScheduledEventId::from_big_decimal(&discord_event.discord_event_id)?;

        // get all meetup events where:
        // - the collection hash matches
        // - the meetup group the event belongs to is being tracked by the guild
        let meetup_events = sqlx::query_as!(
            DBMeetupEvent,
            r#"
                SELECT me.*
                FROM meetup_events AS me
                INNER JOIN meetup_groups_guilds AS mgg
                ON me.meetup_group_name = mgg.group_name
                WHERE me.weekly_collection_hash = $1
                AND mgg.guild_id = $2
            "#,
            collection_hash,
            BigDecimal::from(guild_id.get())
        )
        .fetch_all(pool)
        .await?;

        // if no linked meetup events, delete discord event.
        if meetup_events.len() == 0 {
            // no meetup events: delete discord event
            manage_scheduled_event(
                &ctx,
                ManageType::Delete(scheduled_event_id),
                Vec::new(),
                guild_id,
                pool,
            )
            .await?;
            delete_count += 1;
            continue;
        }

        let Some(first_linked_meetup_event) = first_linked_meetup_event else {
            continue;
        };

        // fetch all duplicate meetup events where:
        // - the dup hash matches
        // - the meetup group the events belong to are being tracked by the guild
        let existing_duplicates = sqlx::query_as!(
            DBMeetupEvent,
            r#"
                SELECT me.*
                FROM meetup_events AS me
                INNER JOIN meetup_groups_guilds AS mgg
                ON me.meetup_group_name = mgg.group_name
                WHERE me.duplicate_event_hash = $1
                AND mgg.guild_id = $2
                ORDER BY me.start_time ASC, me.meetup_group_name ASC
            "#,
            first_linked_meetup_event.duplicate_event_hash,
            BigDecimal::from(guild_id.get())
        )
        .fetch_all(pool)
        .await?;

        // TODO: move to `manage_scheduled_event` Edit()

        // delete all connections (less cumbersome than being picky, though slower(?))
        sqlx::query!(
            "DELETE FROM discord_events_meetup_events WHERE discord_event_id = $1",
            discord_event.discord_event_id
        )
        .execute(pool)
        .await?;

        // tie next upcoming duplicate events to the discord event
        // TODO: move to `manage_scheduled_event` Edit()
        for dup in &existing_duplicates {
            sqlx::query!(
                r#"
                    INSERT INTO discord_events_meetup_events (
                        discord_event_id,
                        meetup_event_id
                    ) VALUES ($1, $2)
                "#,
                discord_event.discord_event_id,
                dup.meetup_event_id,
            )
            .execute(pool)
            .await?;
        }

        // update duplicate hash tied to discord event if needed
        // TODO: move to `manage_scheduled_event` Edit()
        if discord_event.duplicate_event_hash != first_linked_meetup_event.duplicate_event_hash {
            sqlx::query!(
                "UPDATE discord_events SET duplicate_event_hash = $1 WHERE discord_event_id = $2",
                first_linked_meetup_event.duplicate_event_hash,
                discord_event.discord_event_id
            )
            .execute(pool)
            .await?;
        }

        manage_scheduled_event(
            &ctx,
            ManageType::Edit(scheduled_event_id),
            existing_duplicates,
            guild_id,
            pool,
        )
        .await?;
        update_count += 1;
    }
    println!("Synced events in guild with id [{}]", guild_id.get());
    if create_count > 0 {
        println!("[{}] New events", create_count);
    }
    if update_count > 0 {
        println!("[{}] Updated events", update_count);
    }
    if delete_count > 0 {
        println!("[{}] Deleted events", delete_count);
    }
    Ok(())
}

enum ManageType {
    Create,
    Edit(ScheduledEventId),
    Delete(ScheduledEventId),
}

/// builds the first part of a discord event description
/// from a vec of meetup events
fn build_description(meetup_events: &Vec<DBMeetupEvent>) -> String {
    let mut des: Vec<String> = Vec::new();
    des.push("Meetup.com Event Link(s):\n".to_string());
    for event in meetup_events {
        des.push(format!(
            "- https://meetup.com/{}/events/{}\n",
            event.meetup_group_name, event.meetup_event_id
        ));
    }
    des.push(format!("\n{}", meetup_events[0].description));
    let des = des.join("");
    if des.len() >= 1000 {
        return des.split_at(995).0.to_string() + " ...";
    }
    des
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
    match manage_type {
        ManageType::Create => {
            if existing_duplicates.len() == 0 {
                return Ok(());
            }
            let description = build_description(&existing_duplicates);
            let main_event = &existing_duplicates[0];

            let new_discord_event = CreateScheduledEvent::new(
                ScheduledEventType::External,
                main_event.title.to_string(),
                main_event.start_time,
            )
            .description(description)
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
                    collection_hash, 
                    duplicate_event_hash,
                    guild_id
                ) VALUES ($1, $2, $3, $4)
                "#,
                new_scheduled_event_id,
                existing_duplicates[0].weekly_collection_hash,
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
            if existing_duplicates.len() == 0 {
                return Ok(());
            }
            let description = build_description(&existing_duplicates);
            let main_event = &existing_duplicates[0];

            let edit_discord_event = EditScheduledEvent::new()
                .name(main_event.title.to_string())
                .description(description)
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

            sqlx::query!(
                "DELETE FROM discord_events WHERE discord_event_id = $1",
                BigDecimal::from(scheduled_event_id.get())
            )
            .execute(pool)
            .await?;
        }
    };
    Ok(())
}

// Syncs a meetup event pulled from meetup.com with an existing meetup event
// stored in the db
//
// Returns a set of collection hashes of meetup events that were changed
async fn update_meetup_event(
    pool: &sqlx::PgPool,
    now: DateTime<Local>,
    old_event: DBMeetupEvent,
    new_event: MeetupEvent,
) -> Result<HashSet<BigDecimal>, Error> {
    let event_hash = new_event.get_hash();
    if old_event.event_hash == BigDecimal::from(event_hash) {
        sqlx::query!(
            "UPDATE meetup_events SET last_synced = $1 WHERE meetup_event_id = $2",
            now,
            new_event.id
        )
        .execute(pool)
        .await?;
        return Ok(HashSet::new());
    }

    let mut res: HashSet<BigDecimal> = HashSet::new();
    res.insert(BigDecimal::from(old_event.weekly_collection_hash));
    let new_weekly_collection_hash = BigDecimal::from(new_event.get_weekly_collection_hash());
    res.insert(new_weekly_collection_hash.clone());

    // update existing event in db with new data
    sqlx::query!(
        r#"
            UPDATE meetup_events
            SET title = $1, 
                description = $2,
                location = $3,
                event_hash = $4,
                duplicate_event_hash = $5,
                weekly_collection_hash = $6,
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
        new_weekly_collection_hash,
        new_event.start_time,
        new_event.end_time,
        now,
        new_event.id
    )
    .execute(pool)
    .await?;

    Ok(res)
}

// Adds a meetup event pulled from meetup.com to the db
//
// Returns the weekly collection hash of the new meetup event
async fn add_meetup_event(
    pool: &sqlx::PgPool,
    now: DateTime<Local>,
    new_event: MeetupEvent,
) -> Result<BigDecimal, Error> {
    let weekly_collection_hash = BigDecimal::from(new_event.get_weekly_collection_hash());
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
                weekly_collection_hash,
                start_time,
                end_time,
                last_synced
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        new_event.id,
        new_event.group.id,
        new_event.group.url_name,
        new_event.title,
        new_event.description,
        new_event.venue.location,
        BigDecimal::from(new_event.get_hash()),
        BigDecimal::from(new_event.get_dup_hash()),
        weekly_collection_hash,
        new_event.start_time,
        new_event.end_time,
        now
    )
    .execute(pool)
    .await?;

    Ok(weekly_collection_hash)
}

/// Removes meetup events from the db if:
///  - the event has expired
///  - the event was not synced during the most recent resync (indicating either a deleted meetup
///  event or an untracked meetup group)
///
/// This function is designed to be run directly after a sync has occured.
///
/// returns a set of all affected unique collection hashes
async fn clean(pool: &sqlx::PgPool, now: DateTime<Local>) -> Result<HashSet<BigDecimal>, Error> {
    // select a time before the most recent sync but after the sync before that
    let outdated_last_synced = now
        .checked_sub_signed(TimeDelta::from_std(LAST_SYNCED_DELAY).unwrap())
        .unwrap();

    let expired_or_deleted_meetup_event_collection_hashes = sqlx::query!(
        "SELECT DISTINCT weekly_collection_hash FROM meetup_events WHERE end_time <= $1 OR last_synced <= $2",
        now,
        outdated_last_synced,
    )
    .fetch_all(pool)
    .await?;

    sqlx::query!(
        "DELETE FROM meetup_events WHERE end_time <= $1 OR last_synced <= $2",
        now,
        outdated_last_synced
    )
    .execute(pool)
    .await?;

    Ok(expired_or_deleted_meetup_event_collection_hashes
        .iter()
        .map(|r| r.weekly_collection_hash.to_owned())
        .collect())
}

/// Track when a meetup group needs updating
pub async fn toggle_group_update(
    ctx: &Context,
    guild_id: &GuildId,
    meetup_group: &str,
) -> Result<bool, Error> {
    let mut data = ctx.data.write().await;
    let col = data.get_mut::<GroupUpdatesCollection>().unwrap();

    if !col.contains_key(guild_id) {
        col.insert(guild_id.clone(), HashSet::new());
    }

    let guild_groups = col.get_mut(guild_id).unwrap();

    Ok(if guild_groups.contains(meetup_group) {
        guild_groups.remove(meetup_group)
    } else {
        guild_groups.insert(meetup_group.to_string())
    })
}

/// clear tracked group updates for a specific guild
async fn clear_group_updates(ctx: &Context, guild_id: &GuildId) -> Result<bool, Error> {
    let mut data = ctx.data.write().await;
    let col = data.get_mut::<GroupUpdatesCollection>().unwrap();

    if !col.contains_key(guild_id) {
        return Ok(false);
    }

    let _ = col.insert(guild_id.clone(), HashSet::new());

    Ok(true)
}

async fn merge_tracking_updates(
    ctx: &Context,
    pool: &sqlx::PgPool,
    guild_id: &GuildId,
    updates: &HashSet<BigDecimal>,
) -> Result<HashSet<BigDecimal>, Error> {
    let mut collective_updates: HashSet<BigDecimal> = HashSet::new();
    {
        let data = ctx.data.write().await;
        let col = data.get::<GroupUpdatesCollection>().unwrap();
        let groups: HashSet<String> = if col.contains_key(&guild_id) {
            col.get(&guild_id).unwrap().clone()
        } else {
            HashSet::new()
        };
        for group in &groups {
            let hashes = sqlx::query!(
                r#"
                    SELECT DISTINCT weekly_collection_hash
                    FROM meetup_events
                    WHERE meetup_group_name = $1
                "#,
                group
            )
            .fetch_all(pool)
            .await?;
            collective_updates.extend(
                hashes
                    .iter()
                    .map(|h| h.weekly_collection_hash.to_owned())
                    .collect::<HashSet<BigDecimal>>(),
            );
        }
    }
    collective_updates.extend(updates.clone());
    Ok(collective_updates)
}

/// runs on bot startup: determine the bots active guilds, populates db as required
pub async fn populate_db_guilds(ctx: &Context, pool: &sqlx::PgPool) -> Result<(), Error> {
    let active_guilds = util::fetch_all_active_guilds(ctx).await;

    for guild in active_guilds {
        let guild_id = BigDecimal::from(guild.id.get());
        let _was_added = add_guild_to_db(pool, guild_id).await?;
    }

    Ok(())
}

/// add guild to db if required
pub async fn add_guild_to_db(pool: &sqlx::PgPool, guild_id: BigDecimal) -> Result<bool, Error> {
    let has_guild = sqlx::query!("SELECT * FROM guilds WHERE guild_id = $1", guild_id)
        .fetch_optional(pool)
        .await?;

    if let None = has_guild {
        sqlx::query!("INSERT INTO guilds (guild_id) VALUES ($1)", guild_id)
            .execute(pool)
            .await?;
        return Ok(true);
    }
    Ok(false)
}

/// delete guild from db if required
pub async fn remove_guild_from_db(
    pool: &sqlx::PgPool,
    guild_id: BigDecimal,
) -> Result<bool, Error> {
    let has_guild = sqlx::query!("SELECT * FROM guilds WHERE guild_id = $1", guild_id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = has_guild {
        sqlx::query!("DELETE FROM guilds WHERE guild_id = $1", r.guild_id)
            .execute(pool)
            .await?;
        return Ok(true);
    }
    Ok(false)
}

/// Rust representation of sql `discord_events` table.
#[derive(Debug)]
#[allow(unused)]
struct DBDiscordEvent {
    discord_event_id: BigDecimal,
    collection_hash: BigDecimal,
    duplicate_event_hash: BigDecimal,
    guild_id: BigDecimal,
}

/// Rust representation of sql `meetup_events` table.
#[derive(Debug, Clone)]
#[allow(unused)]
struct DBMeetupEvent {
    meetup_event_id: String,
    meetup_group_id: String,
    meetup_group_name: String,
    title: String,
    description: String,
    location: String,
    event_hash: BigDecimal,
    duplicate_event_hash: BigDecimal,
    weekly_collection_hash: BigDecimal,
    start_time: DateTime<FixedOffset>,
    end_time: DateTime<FixedOffset>,
    last_synced: DateTime<FixedOffset>,
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
        #[allow(unused)]
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
