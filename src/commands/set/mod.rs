//! src/commands/set/mod.rs

pub mod bot_access_role;

use crate::{Context, Error};

use crate::commands::_helper;

#[poise::command(slash_command, rename = "set", check = "_helper::has_access")]
pub async fn command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
