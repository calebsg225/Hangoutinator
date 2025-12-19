//! src/commands/meetup/mod.rs

pub mod list;
pub mod track;
pub mod untrack;

use crate::{Context, Error};

use crate::commands::_util;

#[poise::command(slash_command, rename = "meetup", check = "_util::has_access")]
pub async fn command(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
