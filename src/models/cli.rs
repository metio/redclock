// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::completer;
use crate::cli::parser;
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::InfoLevel;
use serde::{Deserialize, Serialize};

/// Redmine time-tracker
#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<InfoLevel>,
}

#[derive(Subcommand)]
pub enum Commands {
    // /// Manage redclock configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Track time
    Track {
        #[command(subcommand)]
        command: TimeTrackCommands,
    },

    /// Manage servers
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Get a configuration value
    Get(ConfigGetArgs),

    /// Set a configuration value
    Set(ConfigSetArgs),
}

#[derive(Args)]
pub struct ConfigGetArgs {
    /// Name of the configuration option to get
    pub option: ConfigurationOption,
}

#[derive(Args)]
pub struct ConfigSetArgs {
    /// Name of the configuration option to set
    pub option: ConfigurationOption,

    /// Value to set the configuration option to
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, clap::ValueEnum)]
pub enum ConfigurationOption {
    ActivitiesFetchIntervalSeconds,
    ProjectsFetchIntervalSeconds,
    IssuesFetchIntervalSeconds,
    FetchRetriesMaximum,
}

#[derive(Subcommand)]
pub enum TimeTrackCommands {
    /// Start tracking time
    Start(TrackStartArgs),

    /// Stop tracking time
    Stop(TrackStopArgs),

    /// Show currently tracking project/issue
    Show(TrackShowArgs),
}

#[derive(Args)]
pub struct TrackStartArgs {
    #[command(flatten)]
    pub server_selection: ServerSelectionArgs,

    /// The activity being performed
    #[arg(short, long)]
    pub activity: Option<String>,

    /// The project being worked-on
    #[arg(short, long, conflicts_with = "issue")]
    pub project: Option<String>,

    /// The issue being worked-on
    #[arg(short, long, conflicts_with = "project")]
    pub issue: Option<String>,

    /// The comment for the new time entry
    #[arg(short, long, conflicts_with = "project")]
    pub comment: Option<String>,
}

#[derive(Args)]
pub struct TrackStopArgs {
    #[command(flatten)]
    pub server_selection: ServerSelectionArgs,

    /// The path of the secret within the selected store
    pub secret_path: Option<String>,
}

#[derive(Args)]
pub struct TrackShowArgs {
    #[command(flatten)]
    pub server_selection: ServerSelectionArgs,
}

#[derive(Subcommand)]
pub enum ServerCommands {
    /// Adds a new server
    Add(ServerAddArgs),

    /// List all available servers
    List(ServerListArgs),

    /// Remove an existing server
    Remove(ServerRemoveArgs),

    /// Mark a server as default
    SetDefault(ServerSetDefaultArgs),
}

#[derive(Args)]
pub struct ServerAddArgs {
    /// The name for the new server. If not set, will use the URL as name
    #[arg(short, long)]
    pub name: Option<String>,

    #[command(flatten)]
    pub token: ServerToken,

    /// Whether the new server should be the default server
    #[arg(short, long)]
    pub default: bool,

    /// The URL of the server
    pub url: String,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct ServerToken {
    /// The API token to use
    #[arg(short, long, conflicts_with = "command")]
    pub token: Option<String>,

    /// The command to execute to retrieve the API token to use
    #[arg(short, long, conflicts_with = "token")]
    pub command: Option<String>,
}

#[derive(Args)]
pub struct ServerRemoveArgs {
    #[command(flatten)]
    pub server_selection: ServerSelectionArgs,
}

#[derive(Args)]
pub struct ServerSetDefaultArgs {
    /// The name of the server to use as default
    #[arg(value_parser = parser::server_name)]
    pub name: String,
}

#[derive(Args)]
pub struct ServerListArgs {}

#[derive(Args)]
pub struct ServerSelectionArgs {
    /// Optional name of server to use. Defaults to the default server or the
    /// first one defined in the local user configuration
    #[arg(short, long, add = completer::server_name(), value_parser = parser::server_name)]
    pub server: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
