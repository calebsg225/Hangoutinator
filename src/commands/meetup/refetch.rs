//! src/commands/meetup/refetch.rs

use crate::commands::_util as util;
use crate::features::event_manager;
use crate::{Context, Error};

/// Force-refetch meetup.com events and sync them with all discord events
#[poise::command(slash_command, rename = "refetch", global_cooldown = 300)]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let pool1 = ctx.data().pool.clone();
    let ctx1 = ctx.serenity_context().clone();
    tokio::spawn(async move {
        if let Err(e) = event_manager::sync_meetup_discord_events(&ctx1, &pool1).await {
            println!("Could not fetch/sync events. Error: {}", e);
        }
    });
    util::send_reply(&ctx, true, "Refetching.").await?;
    Ok(())
}
