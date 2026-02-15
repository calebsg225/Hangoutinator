//! src/commands/meetup/resync.rs

use chrono::Local;

use crate::commands::_util as util;
use crate::features::event_manager;
use crate::{Context, Error};

/// Resync all discord events with meetup.com data already stored in the database.
///
/// This command will not fetch new data from meetup.com.
#[poise::command(slash_command, rename = "resync", global_cooldown = 180)]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        util::send_reply(&ctx, true, "You are not in a guild!").await?;
        return Ok(());
    };
    let pool1 = ctx.data().pool.clone();
    let ctx1 = ctx.serenity_context().clone();
    tokio::spawn(async move {
        let now = Local::now();
        let hashes = event_manager::get_all_guild_collection_hashes(&pool1, &guild_id)
            .await
            .expect("Could not fetch hashes.");
        if let Err(e) =
            event_manager::sync_guild_events(&ctx1, &pool1, now, &hashes, guild_id, true).await
        {
            println!("Could not sync events. Error: {}", e);
        }
    });
    util::send_reply(&ctx, true, "Resyncing. Note that since `/meetup resync` does not fetch new data from meetup.com, meetup group tracking changes may not be reflected.").await?;
    Ok(())
}
