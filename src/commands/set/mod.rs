//! src/commands/set/mod.rs

pub mod bot_access_role;

use crate::{Context, Error};

use crate::commands::_util;

#[poise::command(slash_command, rename = "set", check = "_util::has_access")]
pub async fn command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
