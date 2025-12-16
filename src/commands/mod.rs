//! src/commands/mod.rs

use crate::{Data, Error};

mod command_auth;
mod ping;
mod set;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        ping::command(),
        poise::Command {
            subcommands: vec![set::bot_access_role::command()],
            ..set::command()
        },
    ]
}
