//! src/commands/mod.rs

use crate::{Data, Error};

mod _util;
mod meetup;
mod set;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        poise::Command {
            subcommands: vec![
                set::bot_access_role::command(),
                set::welcome_role::command(),
                set::welcome_channel::command(),
            ],
            ..set::command()
        },
        poise::Command {
            subcommands: vec![
                meetup::resync::command(),
                meetup::list::command(),
                meetup::track::command(),
                meetup::untrack::command(),
            ],
            ..meetup::command()
        },
    ]
}
