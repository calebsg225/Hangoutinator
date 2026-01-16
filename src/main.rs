//! src/main.rs

use std::{collections::HashMap, env};

use poise::serenity_prelude as serenity;

use serenity::{
    Client,
    all::{GatewayIntents, OnlineStatus},
};

mod commands;
mod event_handler;
mod features;
mod meetup;

struct Data {
    pool: sqlx::PgPool,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    // load env vars
    // This will not panic.
    dotenv::dotenv().unwrap_or_default();

    let token = env::var("TOKEN").expect("Expected a TOKEN in the environment.");

    // connect to database
    // TODO: pull db data from env
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::postgres::PgConnectOptions::new()
                .host("database")
                .username("postgres")
                .password("foobar")
                .port(5432),
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
        .status(OnlineStatus::Idle)
        .type_map_insert::<features::welcome_role::UnverifiedMemberCollection>(HashMap::default())
        .await
        .expect("Could not create client");

    if let Err(why) = client.start_autosharded().await {
        println!("Client error: {why:?}");
    }
}
