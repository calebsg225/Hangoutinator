//! src/commands/mod.rs

use crate::{Data, Error};

mod _helper;
mod meetup;
mod ping;
mod set;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        ping::command(),
        poise::Command {
            subcommands: vec![set::bot_access_role::command()],
            ..set::command()
        },
        poise::Command {
            subcommands: vec![
                meetup::list::command(),
                meetup::track::command(),
                meetup::untrack::command(),
            ],
            ..meetup::command()
        },
    ]
}
