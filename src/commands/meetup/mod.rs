//! src/commands/meetup/mod.rs

pub mod list;
pub mod refetch;
pub mod resync;
pub mod track;
pub mod untrack;

use crate::{Context, Error};

use crate::commands::_util;

#[poise::command(
    slash_command,
    rename = "meetup",
    check = "_util::has_access",
    member_cooldown = 1,
    ephemeral = true
)]
pub async fn command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
