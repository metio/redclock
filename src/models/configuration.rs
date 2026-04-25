// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{constants, environment_variables};
use crate::redmine::client::RedmineHttpClient;
use anyhow::{Context, Result};
use redmine_api::api::Redmine;
use serde::{Deserialize, Serialize};
use std::env::var_os;
use std::path::{absolute, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// All registered servers the user has configured on their system
    pub servers: Vec<ServerRegistration>,

    /// The default server to use when no server name was specified
    pub default_server: Option<String>,

    //     PushIntervalSeconds,
    //     FetchRetriesMaximum
    /// The cache duration for activities
    pub activities_fetch_interval_seconds: Option<u64>,

    /// The cache duration for projects
    pub projects_fetch_interval_seconds: Option<u64>,

    /// The cache duration for issues
    pub issues_fetch_interval_seconds: Option<u64>,

    /// The cache duration for issues
    pub fetch_retries_maximum: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ServerRegistration {
    /// The name of the server
    pub name: String,

    /// The URL of the server
    pub url: String,

    /// The API key to access the server
    pub api_key: Option<String>,

    /// The command to execute to get the API key
    pub api_key_command: Option<String>,
}

impl Configuration {
    pub fn load_configuration() -> Result<Self> {
        confy::load_path(Self::config_path()?).context("Could not load configuration")
    }

    pub fn save_configuration(&self) -> Result<()> {
        confy::store_path(Self::config_path()?, self).context("Could not store configuration")
    }

    fn config_path() -> Result<PathBuf> {
        var_os(environment_variables::REDCLOCK_CONFIG).map_or_else(
            || {
                confy::get_configuration_file_path(constants::APPLICATION_NAME, "config")
                    .context("Could not determine configuration path")
            },
            |path| {
                absolute(PathBuf::from(path))
                    .context("Could not resolve absolute path to configuration")
            },
        )
    }

    pub fn data_path() -> Result<PathBuf> {
        var_os(environment_variables::REDCLOCK_DATA).map_or_else(
            || {
                directories::ProjectDirs::from("wtf.metio", "metio", "redclock")
                    .map(|proj_dirs| proj_dirs.data_dir().to_path_buf())
                    .context("Could not determine data directory")
            },
            |path| {
                absolute(PathBuf::from(path))
                    .context("Could not resolve absolute path to data directory")
            },
        )
    }

    pub fn tracking_file_path(server_name: &str) -> Result<PathBuf> {
        Ok(Self::data_path()?.join(server_name).join("tracking.toml"))
    }

    pub fn cache_path() -> Result<PathBuf> {
        var_os(environment_variables::REDCLOCK_CACHE).map_or_else(
            || {
                directories::ProjectDirs::from("wtf.metio", "metio", "redclock")
                    .map(|proj_dirs| proj_dirs.cache_dir().to_path_buf())
                    .context("Could not determine cache directory")
            },
            |path| {
                absolute(PathBuf::from(path))
                    .context("Could not resolve absolute path to cache directory")
            },
        )
    }

    pub fn cache_directory(server_name: &str) -> Result<PathBuf> {
        Ok(Self::cache_path()?.join(server_name))
    }

    pub fn add_server(&mut self, registration: ServerRegistration) -> Result<()> {
        self.servers.push(registration);
        self.save_configuration()
    }

    pub fn remove_server(&mut self, store_name: &str) -> Result<()> {
        self.default_server
            .take_if(|value| value.eq_ignore_ascii_case(store_name));
        self.servers
            .retain(|store| !store.name.eq_ignore_ascii_case(store_name));
        self.save_configuration()
    }

    fn default_server_name(&self) -> Option<String> {
        var_os(environment_variables::REDCLOCK_DEFAULT_STORE).map_or_else(
            || self.default_server.clone(),
            |value| value.into_string().ok(),
        )
    }

    pub fn set_default_server(&mut self, server_name: &str) -> Result<()> {
        self.default_server = Some(server_name.to_owned());
        self.save_configuration()
    }

    pub fn all_server_names(&self) -> Vec<String> {
        let mut names = vec![];
        for server in &self.servers {
            names.push(server.name.clone());
        }
        names
    }

    pub fn select_server(&self, server_name: Option<&String>) -> Option<&ServerRegistration> {
        server_name
            .cloned()
            .or_else(|| self.default_server_name())
            .map_or_else(
                || self.servers.first(),
                |name| self.find_server(name.as_str()),
            )
    }

    pub fn find_server(&self, server_name: &str) -> Option<&ServerRegistration> {
        self.servers
            .iter()
            .find(|store| store.name.eq_ignore_ascii_case(server_name))
    }

    pub fn create_redmine_client(
        &self,
        server_registration: &ServerRegistration,
    ) -> Result<RedmineHttpClient> {
        let client = reqwest::blocking::Client::builder()
            .retry(
                reqwest::retry::for_host(server_registration.url.clone())
                    .max_retries_per_request(self.fetch_retries_maximum.unwrap_or(3)),
            )
            .tls_backend_rustls()
            .build()?;
        let redmine = Redmine::new(
            client,
            reqwest::Url::parse(&server_registration.url)?,
            &server_registration.api_key()?,
        )?;

        Ok(RedmineHttpClient::new(
            redmine,
            Self::cache_directory(&server_registration.name)?,
            self.activities_fetch_interval_seconds.unwrap_or(60 * 60 * 24 * 7), // 1 week
            self.projects_fetch_interval_seconds.unwrap_or(60 * 60 * 24), // 1 day
            self.issues_fetch_interval_seconds.unwrap_or(60 * 60), // 1 hour
        ))
    }
}

impl ServerRegistration {
    fn api_key(&self) -> Result<String> {
        if let Some(command) = self.api_key_command.clone() && !command.is_empty() {
            if let Some(parts) = shlex::split(&command) {
                let binary = &parts[0];
                let args = &parts[1..];

                Ok(duct::cmd(binary, args)
                    .stdout_capture()
                    .read()?)
            } else {
                anyhow::bail!("Cannot parse command: {command:?}");
            }
        } else if let Some(token) = self.api_key.clone() && !token.is_empty() {
            Ok(token)
        } else {
            anyhow::bail!("No API key specified");
        }
    }
}
