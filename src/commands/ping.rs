//! src/commands/ping.rs

use poise::serenity_prelude as seren;
use seren::all::Mentionable;

use crate::{Context, Error};

use crate::commands::_util;

#[poise::command(
    slash_command,
    rename = "ping",
    identifying_name = "iping",
    check = "_util::has_access"
)]
pub async fn command(
    ctx: Context<'_>,
    #[description = "User to ping"] user: Option<seren::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let res = format!("Pong {}!", u.mention());
    ctx.say(res).await?;
    Ok(())
}
