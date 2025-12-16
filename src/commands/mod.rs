//! src/commands/mod.rs

use crate::{Data, Error};

mod ping;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![ping::command()]
}
