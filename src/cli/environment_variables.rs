// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::constants;

pub const REDCLOCK_CONFIG: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_config")
);

pub const REDCLOCK_DATA: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_data")
);

pub const REDCLOCK_CACHE: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_cache")
);

pub const REDCLOCK_DEFAULT_STORE: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_default_server")
);
