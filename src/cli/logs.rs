// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use log::info;

pub fn server_add_success(server_name: &str) {
    info!("Server '{server_name}' added");
}

pub fn server_set_default(server_name: &str) {
    info!("Server '{server_name}' is now the default");
}

pub fn server_remove_success(server_name: &str) {
    info!("Server '{server_name}' removed");
}
