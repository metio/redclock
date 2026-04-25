// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use anyhow::Error;
use redmine_api::api::Redmine;
use redmine_api::api::enumerations::{
    ListTimeEntryActivities, TimeEntryActivitiesWrapper, TimeEntryActivity,
};
use redmine_api::api::issues::{Issue, IssueStatusFilter, ListIssues};
use redmine_api::api::projects::{ListProjects, Project};
use redmine_api::api::time_entries::{CreateTimeEntry, TimeEntry, TimeEntryWrapper};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait RedmineClient {
    fn get_all_activities(&self) -> anyhow::Result<Vec<TimeEntryActivity>>;
    fn get_all_projects(&self) -> anyhow::Result<Vec<Project>>;
    fn get_all_open_issues(&self) -> anyhow::Result<Vec<Issue>>;
    fn track_project_time(
        &self,
        activity_id: u64,
        project_id: u64,
        option: Option<&String>,
        elapsed_hours: f64,
    ) -> anyhow::Result<()>;
    fn track_issue_time(
        &self,
        activity_id: u64,
        issue_id: u64,
        option: Option<&String>,
        elapsed_hours: f64,
    ) -> anyhow::Result<()>;
}

pub struct RedmineHttpClient {
    redmine: Redmine,
    cache_directory: PathBuf,
    activities_fetch_interval_seconds: u64,
    projects_fetch_interval_seconds: u64,
    issues_fetch_interval_seconds: u64,
}

impl RedmineHttpClient {
    pub const fn new(
        redmine: Redmine,
        cache_directory: PathBuf,
        activities_fetch_interval_seconds: u64,
        projects_fetch_interval_seconds: u64,
        issues_fetch_interval_seconds: u64,
    ) -> Self {
        Self {
            redmine,
            cache_directory,
            activities_fetch_interval_seconds,
            projects_fetch_interval_seconds,
            issues_fetch_interval_seconds,
        }
    }

    fn should_read_from_cache(
        cache_timestamp_path: &PathBuf,
        fetch_interval_seconds: u64,
    ) -> anyhow::Result<bool> {
        let cache_timestamp = fs::read_to_string(cache_timestamp_path)?;
        let last_fetched = cache_timestamp.parse::<u64>()?;
        let current_time = Self::current_time_in_seconds()?;

        Ok(current_time - last_fetched < fetch_interval_seconds)
    }

    fn current_time_in_seconds() -> Result<u64, Error> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
    }
}

impl RedmineClient for RedmineHttpClient {
    fn get_all_activities(&self) -> anyhow::Result<Vec<TimeEntryActivity>> {
        let cache_path = self.cache_directory.join("activities.json");
        let cache_timestamp_path = self.cache_directory.join("activities.last_fetched");

        if cache_path.exists()
            && cache_timestamp_path.exists()
            && Self::should_read_from_cache(
                &cache_timestamp_path,
                self.activities_fetch_interval_seconds,
            )?
        {
            // read from cache
            let cache_content = fs::read_to_string(&cache_path)?;
            Ok(serde_json::from_str(&cache_content)?)
        } else {
            // fetch from server
            let endpoint = ListTimeEntryActivities::builder().build()?;
            let activities = self
                .redmine
                .json_response_body::<_, TimeEntryActivitiesWrapper<TimeEntryActivity>>(&endpoint)
                .map(|response| response.time_entry_activities)?;

            // write to cache
            if let Some(parent) = &cache_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let cache_content = serde_json::to_string(&activities)?;
            fs::write(&cache_path, &cache_content)?;
            fs::write(
                &cache_timestamp_path,
                format!("{}", Self::current_time_in_seconds()?),
            )?;

            Ok(activities)
        }
    }

    fn get_all_projects(&self) -> anyhow::Result<Vec<Project>> {
        let cache_path = self.cache_directory.join("projects.json");
        let cache_timestamp_path = self.cache_directory.join("projects.last_fetched");

        if cache_path.exists()
            && cache_timestamp_path.exists()
            && Self::should_read_from_cache(
                &cache_timestamp_path,
                self.projects_fetch_interval_seconds,
            )?
        {
            // read from cache
            let cache_content = fs::read_to_string(&cache_path)?;
            Ok(serde_json::from_str(&cache_content)?)
        } else {
            // fetch from server
            let endpoint = ListProjects::builder().build()?;
            let projects = self
                .redmine
                .json_response_body_all_pages::<_, Project>(&endpoint)?;

            // write to cache
            if let Some(parent) = &cache_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let cache_content = serde_json::to_string(&projects)?;
            fs::write(&cache_path, &cache_content)?;
            fs::write(
                &cache_timestamp_path,
                format!("{}", Self::current_time_in_seconds()?),
            )?;

            Ok(projects)
        }
    }

    fn get_all_open_issues(&self) -> anyhow::Result<Vec<Issue>> {
        let cache_path = self.cache_directory.join("issues.json");
        let cache_timestamp_path = self.cache_directory.join("issues.last_fetched");

        if cache_path.exists()
            && cache_timestamp_path.exists()
            && Self::should_read_from_cache(
                &cache_timestamp_path,
                self.issues_fetch_interval_seconds,
            )?
        {
            // read from cache
            let cache_content = fs::read_to_string(&cache_path)?;
            Ok(serde_json::from_str(&cache_content)?)
        } else {
            // fetch from server
            let endpoint = ListIssues::builder()
                .status_id(IssueStatusFilter::Open)
                .build()?;
            let issues = self
                .redmine
                .json_response_body_all_pages::<_, Issue>(&endpoint)?;

            // write to cache
            if let Some(parent) = &cache_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let cache_content = serde_json::to_string(&issues)?;
            fs::write(&cache_path, &cache_content)?;
            fs::write(
                &cache_timestamp_path,
                format!("{}", Self::current_time_in_seconds()?),
            )?;

            Ok(issues)
        }
    }

    fn track_project_time(
        &self,
        activity_id: u64,
        project_id: u64,
        comment: Option<&String>,
        elapsed_hours: f64,
    ) -> anyhow::Result<()> {
        let endpoint = CreateTimeEntry::builder()
            .activity_id(activity_id)
            .project_id(project_id)
            .hours(elapsed_hours)
            .comments(comment.cloned().unwrap_or_default().into())
            .build()?;
        self.redmine
            .json_response_body::<_, TimeEntryWrapper<TimeEntry>>(&endpoint)?;
        Ok(())
    }

    fn track_issue_time(
        &self,
        activity_id: u64,
        issue_id: u64,
        comment: Option<&String>,
        elapsed_hours: f64,
    ) -> anyhow::Result<()> {
        let endpoint = CreateTimeEntry::builder()
            .activity_id(activity_id)
            .issue_id(issue_id)
            .hours(elapsed_hours)
            .comments(comment.cloned().unwrap_or_default().into())
            .build()?;
        self.redmine
            .json_response_body::<_, TimeEntryWrapper<TimeEntry>>(&endpoint)?;
        Ok(())
    }
}
