//! src/commands/set/mod.rs

pub mod logging_channel;
pub mod logging_level;
pub mod welcome_channel;
pub mod welcome_role;

use crate::{Context, Error};

use crate::commands::_util;

/// Set roles/channels for various bot functions
#[poise::command(
    slash_command,
    rename = "set",
    check = "_util::has_access",
    member_cooldown = 1
)]
pub async fn command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
