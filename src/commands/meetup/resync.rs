//! src/commands/meetup/resync.rs

use crate::commands::_util as util;
use crate::features::event_manager::sync_meetup_discord_events;
use crate::{Context, Error};

const EPHEMERAL: bool = true;

/// Force-refetch meetup.com events and resync them with this servers discord events
#[poise::command(slash_command, rename = "resync", global_cooldown = 65)]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        util::send_reply(&ctx, EPHEMERAL, "You are not in a guild!").await?;
        return Ok(());
    };
    let pool1 = ctx.data().pool.clone();
    let ctx1 = ctx.serenity_context().clone();
    tokio::spawn(async move {
        if let Err(e) = sync_meetup_discord_events(&ctx1, &pool1, Some(guild_id)).await {
            println!("Could not sync events. Error: {}", e);
        }
    });
    util::send_reply(&ctx, EPHEMERAL, "Resyncing has begun.").await?;
    Ok(())
}
