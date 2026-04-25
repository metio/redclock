// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use skim::options::SkimOptionsBuilder;
use skim::tui::options::TuiLayout;
use skim::{SkimItem, SkimOptions};
use std::borrow::Cow;

pub struct RedmineActivityItem {
    pub activity_id: u64,
    pub activity_name: String,
}

pub struct RedmineProjectIssueItem {
    pub project_id: Option<u64>,
    pub project_identifier: Option<String>,
    pub project_name: Option<String>,
    pub issue_id: Option<u64>,
    pub issue_title: Option<String>,
    pub variant: RedmineProjectIssueItemVariant,
}

#[derive(PartialEq, Eq)]
pub enum RedmineProjectIssueItemVariant {
    Project,
    Issue,
}

impl SkimItem for RedmineActivityItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::from(format!(
            "Activity (#{}): {}",
            self.activity_id, self.activity_name
        ))
    }
}

impl SkimItem for RedmineProjectIssueItem {
    fn text(&self) -> Cow<'_, str> {
        if self.variant == RedmineProjectIssueItemVariant::Issue {
            Cow::from(format!(
                "Issue (#{}): {}",
                self.issue_id.unwrap_or_default(),
                self.issue_title.clone().unwrap_or_default()
            ))
        } else {
            Cow::from(format!(
                "Project (#{}) {}: {}",
                self.project_id.unwrap_or_default(),
                self.project_identifier.clone().unwrap_or_default(),
                self.project_name.clone().unwrap_or_default()
            ))
        }
    }
}

pub fn create_skim_options(prompt: &str) -> SkimOptions {
    SkimOptionsBuilder::default()
        .height("100%")
        .cycle(true)
        .multi(false)
        .prompt(prompt.trim().to_string() + " ")
        .layout(TuiLayout::Reverse)
        .build()
        .unwrap_or_default()
}
