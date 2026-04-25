// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use anyhow::Context;
use anyhow::Result;

use crate::models::configuration::Configuration;

pub fn server_name(input: &str) -> Result<String> {
    let configuration =
        Configuration::load_configuration().context("Could not load configuration")?;
    let names = configuration.all_server_names();

    if names.contains(&input.to_owned()) {
        Ok(input.to_owned())
    } else {
        anyhow::bail!("Server with name '{input}' does not exist in configuration")
    }
}
