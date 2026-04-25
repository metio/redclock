// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct StartTime {
    pub server: String,
    pub activity_id: u64,
    pub activity: String,
    pub project_id: Option<u64>,
    pub project: Option<String>,
    pub issue_id: Option<u64>,
    pub issue: Option<String>,
    pub comment: Option<String>,
    pub start: chrono::DateTime<chrono::Utc>,
}
