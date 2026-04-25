// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

mod cli;
mod commands;
mod models;
mod redmine;

use anyhow::{Result};
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use human_panic::{setup_panic, Metadata};
use crate::commands::{config, servers, time_trackers};
use crate::models::cli::{Cli, Commands};
use crate::models::configuration::Configuration;


fn main() -> Result<()> {
    setup_panic!(
        Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .authors("metio.wtf <redclock@metio.wtf>")
            .homepage("https://github.com/metio/redclock")
            .support("- Open a support request by creating a ticket on GitHub")
    );

    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    env_logger::builder()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .format_level(false)
        .format_module_path(false)
        .format_source_path(false)
        .format_target(false)
        .init();
    let configuration = Configuration::load_configuration()?;

    match &cli.command {
        Commands::Config { command } => config::dispatch(command, configuration),
        Commands::Server { command } => servers::dispatch(command, configuration),
        Commands::Track { command } => time_trackers::dispatch(command, &configuration),
    }
}
