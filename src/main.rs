mod cli;
mod commands;
mod db;
mod git_utils;
mod types;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Set {
            value_type,
            target,
            key,
            value,
        } => commands::set::run(&target, &key, &value, &value_type),

        Commands::Get {
            json,
            with_authorship,
            target,
            key,
        } => commands::get::run(&target, key.as_deref(), json, with_authorship),

        Commands::Rm { target, key } => commands::rm::run(&target, &key),

        Commands::ListPush { target, key, value } => {
            commands::list::run_push(&target, &key, &value)
        }

        Commands::ListPop { target, key, value } => commands::list::run_pop(&target, &key, &value),

        Commands::Serialize => commands::serialize::run(),

        Commands::Materialize { remote } => commands::materialize::run(remote.as_deref()),
    }
}
