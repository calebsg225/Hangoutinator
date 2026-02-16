//! src/commands/meetup/purge.rs

use chrono::Local;

use crate::commands::_util as util;
use crate::features::event_manager;
use crate::{Context, Error};

/// Remove all discord events created by Dave Bot in this discord server.
#[poise::command(slash_command, rename = "purge", guild_cooldown = 300)]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        util::send_reply(&ctx, true, "You are not in a guild!").await?;
        return Ok(());
    };
    let pool1 = ctx.data().pool.clone();
    let ctx1 = ctx.serenity_context().clone();
    let now = Local::now();
    tokio::spawn(async move {
        if let Err(e) = event_manager::execute_meetup_action(
            &ctx1,
            &pool1,
            now,
            event_manager::MeetupAction::Purge(Some(guild_id)),
        )
        .await
        {
            println!("Could not purge discord events. Error: {}", e);
        };
    });
    util::send_reply(
        &ctx,
        true,
        "Purging events now.\nNote: Depending on the number of events, this may take some time.",
    )
    .await?;
    Ok(())
}
