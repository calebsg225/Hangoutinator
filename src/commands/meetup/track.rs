//! src/commands/meetup/track.rs

//use sqlx::types::BigDecimal;

//use crate::commands::_helper as helper;
use crate::{Context, Error};

#[poise::command(slash_command, rename = "track")]
pub async fn command(
    ctx: Context<'_>,
    #[description = "the name of the meetup group to track"] _group_name: String,
) -> Result<(), Error> {
    let _pool = &ctx.data().pool;
    let _guild_id = ctx.guild_id().unwrap().get();
    // - check if the group exists in db. if not, add.
    // - if it does, check if guild is already tracking
    // - if not, track and update events accordingly
    Ok(())
}
