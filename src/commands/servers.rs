// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::logs;
use crate::models::cli::ServerCommands;
use crate::models::configuration::{Configuration, ServerRegistration};

pub fn dispatch(command: &ServerCommands, configuration: Configuration) -> anyhow::Result<()> {
    match command {
        ServerCommands::Add(args) => add(
            configuration,
            &args.url,
            args.name.as_ref(),
            args.token.token.as_ref(),
            args.token.command.as_ref(),
            args.default,
        ),
        ServerCommands::Remove(args) => {
            remove(configuration, args.server_selection.server.as_ref())
        }
        ServerCommands::List(_) => {
            list(&configuration);
            Ok(())
        }
        ServerCommands::SetDefault(args) => set_default(configuration, &args.name),
    }
}

fn add(
    mut configuration: Configuration,
    server_url: &str,
    server_name: Option<&String>,
    token: Option<&String>,
    command: Option<&String>,
    default: bool,
) -> anyhow::Result<()> {
    let name = server_name.cloned().unwrap_or_else(|| {
        server_url
            .replace("https://", "")
            .replace("http://", "")
            .trim_end_matches('/')
            .to_lowercase()
    });

    if configuration.find_server(&name).is_some() {
        anyhow::bail!("Server name already exists. Please use a different name.");
    }

    let registration = ServerRegistration {
        name: name.clone(),
        url: server_url.to_owned(),
        api_key: token.cloned(),
        api_key_command: command.cloned(),
    };

    configuration.add_server(registration)?;
    logs::server_add_success(&name);
    if default {
        set_default(configuration, &name)?;
    }
    Ok(())
}

fn remove(mut configuration: Configuration, server_name: Option<&String>) -> anyhow::Result<()> {
    let name = configuration
        .select_server(server_name)
        .map(|server| server.name.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No server found in configuration. Run 'redclock server add ...' first to add one"
            )
        })?;

    configuration.remove_server(&name)?;
    logs::server_remove_success(&name);
    Ok(())
}

fn list(configuration: &Configuration) {
    for server in &configuration.servers {
        let text = configuration
            .default_server
            .clone()
            .filter(|default| default == &server.name)
            .map_or_else(
                || format!("{}: {}", server.name, server.url),
                |default| format!("{}: {} (default)", default, server.url),
            );

        println!("{text}");
    }
}

fn set_default(mut configuration: Configuration, server_name: &str) -> anyhow::Result<()> {
    configuration.set_default_server(server_name)?;
    logs::server_set_default(server_name);
    Ok(())
}
