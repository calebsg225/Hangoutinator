//! src/features/logging.rs
//!
//! Give discord guilds the ability to output certain info to a discord channel

use crate::{Error, IdExt};
use serenity::all::{ChannelId, Context, CreateMessage};
use sqlx::types::BigDecimal;

pub async fn log(
    ctx: &Context,
    pool: &sqlx::PgPool,
    msg: String,
    log_level: DiscordLogLevel,
) -> Result<(), Error> {
    let guilds_info = sqlx::query_as!(
        GuildLoggingInfo,
        "SELECT logging_channel_id, logging_level FROM guilds WHERE logging_channel_id IS NOT NULL AND logging_level IS NOT NULL"
    )
    .fetch_all(pool)
    .await?;
    for guild_info in guilds_info {
        let ctx1 = ctx.clone();
        let msg1 = msg.clone();
        // check that a logging channel is set
        let channel_id = match guild_info.logging_channel_id {
            Some(id) => ChannelId::from_big_decimal(&id).unwrap(),
            None => return Ok(()),
        };
        // check that the log priority is correct
        if DiscordLogLevel::from(guild_info.logging_level) < log_level {
            return Ok(());
        }
        tokio::spawn(async move {
            // send log
            let _ = channel_id
                .send_message(&ctx1.http, CreateMessage::new().content(msg1))
                .await;
        });
    }
    Ok(())
}

#[derive(PartialOrd, PartialEq, Clone, Debug, poise::ChoiceParameter)]
pub enum DiscordLogLevel {
    None = 0,
    Fatal = 1,
    Error = 2,
    Warn = 3,
    Info = 4,
    Debug = 5,
    All = 6,
}

impl From<Option<i32>> for DiscordLogLevel {
    fn from(i: Option<i32>) -> Self {
        match i {
            Some(i) => match i {
                1 => Self::Fatal,
                2 => Self::Error,
                3 => Self::Warn,
                4 => Self::Info,
                5 => Self::Debug,
                6 => Self::All,
                _ => Self::None,
            },
            None => Self::None,
        }
    }
}

struct GuildLoggingInfo {
    logging_channel_id: Option<BigDecimal>,
    logging_level: Option<i32>,
}
