//! src/commands/meetup/untrack.rs

use crate::{Context, Error};

#[poise::command(slash_command, rename = "untrack")]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
