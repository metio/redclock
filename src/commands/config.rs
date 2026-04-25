// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::models::cli::{ConfigCommands, ConfigurationOption};
use crate::models::configuration::Configuration;

pub fn dispatch(command: &ConfigCommands, configuration: Configuration) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Get(args) => {
            get(&configuration, &args.option);
            Ok(())
        }
        ConfigCommands::Set(args) => set(configuration, &args.option, &args.value),
    }
}

fn get(configuration: &Configuration, option: &ConfigurationOption) {
    match option {
        ConfigurationOption::ActivitiesFetchIntervalSeconds => {
            println!(
                "{}",
                configuration
                    .activities_fetch_interval_seconds
                    .unwrap_or(60 * 60 * 24 * 7)
            );
        }
        ConfigurationOption::ProjectsFetchIntervalSeconds => {
            println!(
                "{}",
                configuration
                    .projects_fetch_interval_seconds
                    .unwrap_or(60 * 60 * 24)
            );
        }
        ConfigurationOption::IssuesFetchIntervalSeconds => {
            println!(
                "{}",
                configuration
                    .issues_fetch_interval_seconds
                    .unwrap_or(60 * 60)
            );
        }
        ConfigurationOption::FetchRetriesMaximum => {
            println!("{}", configuration.fetch_retries_maximum.unwrap_or(3));
        }
    }
}

fn set(
    mut configuration: Configuration,
    option: &ConfigurationOption,
    value: &str,
) -> anyhow::Result<()> {
    match option {
        ConfigurationOption::ActivitiesFetchIntervalSeconds => {
            if value.is_empty() {
                configuration.activities_fetch_interval_seconds = None;
            } else {
                let interval = value.parse::<u64>()?;
                configuration.activities_fetch_interval_seconds = Some(interval);
            }
        }
        ConfigurationOption::ProjectsFetchIntervalSeconds => {
            if value.is_empty() {
                configuration.projects_fetch_interval_seconds = None;
            } else {
                let interval = value.parse::<u64>()?;
                configuration.projects_fetch_interval_seconds = Some(interval);
            }
        }
        ConfigurationOption::IssuesFetchIntervalSeconds => {
            if value.is_empty() {
                configuration.issues_fetch_interval_seconds = None;
            } else {
                let interval = value.parse::<u64>()?;
                configuration.issues_fetch_interval_seconds = Some(interval);
            }
        }
        ConfigurationOption::FetchRetriesMaximum => {
            if value.is_empty() {
                configuration.fetch_retries_maximum = None;
            } else {
                let interval = value.parse::<u32>()?;
                configuration.fetch_retries_maximum = Some(interval);
            }
        }
    }
    configuration.save_configuration()
}
