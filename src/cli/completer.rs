// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use clap_complete::{ArgValueCompleter, CompletionCandidate};

use crate::models::configuration::Configuration;

pub fn server_name() -> ArgValueCompleter {
    ArgValueCompleter::new(|current: &std::ffi::OsStr| {
        Configuration::load_configuration().map_or_else(
            |_| vec![],
            |configuration| {
                let names = configuration.all_server_names();
                current.to_str().map_or_else(
                    || names.iter().map(CompletionCandidate::new).collect(),
                    |value| {
                        names
                            .iter()
                            .filter(|&name| name.starts_with(value))
                            .map(CompletionCandidate::new)
                            .collect()
                    },
                )
            },
        )
    })
}
