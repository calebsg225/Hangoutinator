//! src/main.rs

use std::{collections::HashMap, env};

use serenity::{
    Client,
    all::{GatewayIntents, OnlineStatus},
};

mod event_handler;
mod features;

#[tokio::main]
async fn main() {
    // load env vars
    // This will not panic.
    dotenv::dotenv().unwrap_or_default();

    let token = env::var("TOKEN").expect("Expected a token in the environment.");

    // connect to database
    // TODO: pull db data from env?
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

    // `GUILD_MEMBERS` used to detect role assignment/reassignment
    let intents = GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGES;

    let handler = event_handler::Handler { db_pool: pool };

    // build client
    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .status(OnlineStatus::Idle)
        .type_map_insert::<features::welcome_role::UnverifiedMemberCollection>(HashMap::default())
        .await
        .expect("Could not create client");

    if let Err(why) = client.start_autosharded().await {
        println!("Client error: {why:?}");
    }
}
