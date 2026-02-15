//! src/commands/meetup/refetch.rs

use chrono::Local;

use crate::commands::_util as util;
use crate::features::event_manager;
use crate::{Context, Error};

/// Force-refetch meetup.com events and sync them with all discord events
#[poise::command(slash_command, rename = "refetch", global_cooldown = 300)]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let pool1 = ctx.data().pool.clone();
    let ctx1 = ctx.serenity_context().clone();
    tokio::spawn(async move {
        let now = Local::now();
        if let Err(e) = event_manager::execute_meetup_action(
            &ctx1,
            &pool1,
            now,
            event_manager::MeetupAction::FetchAndSync,
        )
        .await
        {
            println!("Could not fetch/sync events. Error: {}", e);
        }
    });
    util::send_reply(&ctx, true, "Refetching.").await?;
    Ok(())
}
