//! src/main.rs

use std::{collections::HashMap, env};

use poise::serenity_prelude as serenity;

use ::serenity::all::{ChannelId, GuildId, RoleId, ScheduledEventId};
use serenity::{
    Client,
    all::{GatewayIntents /*OnlineStatus*/},
};
use sqlx::types::BigDecimal;

mod commands;
mod event_handler;
mod features;
mod meetup;

struct Data {
    pool: sqlx::PgPool,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

trait Id {}
trait IdExt<T> {
    fn from_big_decimal(big_decimal: &BigDecimal) -> Result<T, Error>;
}

impl<T: Id + From<u64>> IdExt<T> for T {
    fn from_big_decimal(big_decimal: &BigDecimal) -> Result<T, Error> {
        Ok(T::from(big_decimal.to_string().parse::<u64>()?))
    }
}

impl Id for ScheduledEventId {}
impl Id for ChannelId {}
impl Id for RoleId {}
impl Id for GuildId {}

#[tokio::main]
async fn main() {
    // load env vars
    // This will not panic.
    dotenv::dotenv().unwrap_or_default();

    let token = env::var("TOKEN").expect("Expected a TOKEN in the environment.");

    // connect to database
    // TODO: unwraps...
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::postgres::PgConnectOptions::new()
                .host(&env::var("PGHOST").unwrap())
                .username(&env::var("PGUSER").unwrap())
                .password(&env::var("PGPASSWORD").unwrap())
                .port(env::var("PGPORT").unwrap().parse::<u16>().unwrap()),
        )
        .await
        .expect("Could not connect to database.");

    let data = Data { pool: pool.clone() };

    // `GUILD_MEMBERS` used to detect role assignment/reassignment
    let intents = GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES;

    let handler = event_handler::Handler { pool: pool };

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions::<Data, Error> {
            commands: commands::all_commands(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(data)
            })
        })
        .build();

    // build client
    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .framework(framework)
        //.status(OnlineStatus::Idle)
        .type_map_insert::<features::welcome_role::UnverifiedMemberCollection>(HashMap::default())
        .await
        .expect("Could not create client");

    if let Err(why) = client.start_autosharded().await {
        println!("Client error: {why:?}");
    }
}
