//! src/commands/meetup/resync.rs

use crate::commands::_util as util;
use crate::features::event_manager::sync_meetup_discord_events;
use crate::{Context, Error};

// TODO: implement a cooldown (very important for this command)
#[poise::command(slash_command, rename = "resync")]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let pool1 = ctx.data().pool.clone();
    let ctx1 = ctx.serenity_context().clone();
    tokio::spawn(async move {
        if let Err(e) = sync_meetup_discord_events(&ctx1, &pool1).await {
            println!("Could not sync events. Error: {}", e);
        }
    });
    util::send_reply(&ctx, true, "Resyncing has begun.").await?;
    Ok(())
}
