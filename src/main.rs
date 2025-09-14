//! main.rs

use std::env;

use serenity::{
    Client,
    all::{Context, EventHandler, GatewayIntents, Message, Ready},
    async_trait,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending the message: {why:?}");
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // load env vars located at `./.env`
    dotenv::dotenv().expect("Failed to load .env file.");

    let token = env::var("TOKEN").expect("Expected a token in the environment.");
    println!("{token}");

    // set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as the bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // start a single shard, and start listening to events
    //
    // shards will automatically attempt to reconnect, and will preform exponential backoff until
    // it reconnects
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
    println!("Hello, world!");
}
