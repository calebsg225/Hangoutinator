//! src/commands/owner/mod.rs

pub mod set_bot_access_role;

use crate::{Context, Error};

use crate::commands::_util;

/// A collection of commands only accessable to the discord guild owner
#[poise::command(
    slash_command,
    rename = "owner",
    check = "_util::is_guild_owner",
    member_cooldown = 1
)]
pub async fn command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
