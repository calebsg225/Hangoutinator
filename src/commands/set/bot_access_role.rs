//! src/commands/set/bot_access_role.rs

use crate::{Context, Error};

#[poise::command(slash_command, rename = "bot_access_role")]
pub async fn command(ctx: Context<'_>) -> Result<(), Error> {
    //let pool = &ctx.data().pool;
    let res = format!("Set bot access role to ...");
    ctx.say(res).await?;
    Ok(())
}
