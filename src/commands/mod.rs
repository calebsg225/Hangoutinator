//! src/commands/mod.rs

use crate::{Data, Error};

mod _util;
mod meetup;
mod owner;
mod set;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        poise::Command {
            subcommands: vec![owner::set_bot_access_role::command()],
            ..owner::command()
        },
        poise::Command {
            subcommands: vec![
                set::welcome_role::command(),
                set::welcome_channel::command(),
            ],
            ..set::command()
        },
        poise::Command {
            subcommands: vec![
                //meetup::resync::command(),
                meetup::refetch::command(),
                meetup::list::command(),
                meetup::track::command(),
                meetup::untrack::command(),
            ],
            ..meetup::command()
        },
    ]
}
