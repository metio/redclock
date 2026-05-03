// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use anyhow::Error;
use redmine_api::api::Redmine;
use redmine_api::api::enumerations::{
    ListTimeEntryActivities, TimeEntryActivitiesWrapper, TimeEntryActivity,
};
use redmine_api::api::issues::{Issue, IssueStatusFilter, ListIssues};
use redmine_api::api::projects::{ListProjects, Project, ProjectStatusFilter};
use redmine_api::api::time_entries::{CreateTimeEntry, TimeEntry, TimeEntryWrapper};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait RedmineClient {
    fn get_all_activities(&self) -> anyhow::Result<Vec<TimeEntryActivity>>;
    fn get_all_projects(&self) -> anyhow::Result<Vec<Project>>;
    fn get_all_open_issues(&self) -> anyhow::Result<Vec<Issue>>;
    fn track_project_time(
        &self,
        activity_id: u64,
        project_id: u64,
        option: &str,
        elapsed_hours: f64,
    ) -> anyhow::Result<()>;
    fn track_issue_time(
        &self,
        activity_id: u64,
        issue_id: u64,
        option: &str,
        elapsed_hours: f64,
    ) -> anyhow::Result<()>;
}

pub struct RedmineHttpClient {
    redmine: Redmine,
    cache_directory: Option<PathBuf>,
    activities_fetch_interval_seconds: u64,
    projects_fetch_interval_seconds: u64,
    issues_fetch_interval_seconds: u64,
    ignore_cache: bool,
}

impl RedmineHttpClient {
    pub const fn new(
        redmine: Redmine,
        cache_directory: Option<PathBuf>,
        activities_fetch_interval_seconds: u64,
        projects_fetch_interval_seconds: u64,
        issues_fetch_interval_seconds: u64,
        ignore_cache: bool,
    ) -> Self {
        Self {
            redmine,
            cache_directory,
            activities_fetch_interval_seconds,
            projects_fetch_interval_seconds,
            issues_fetch_interval_seconds,
            ignore_cache,
        }
    }

    fn should_read_from_cache(cache_timestamp_path: &PathBuf, fetch_interval_seconds: u64) -> bool {
        let cache_timestamp = fs::read_to_string(cache_timestamp_path).unwrap_or_default();
        let last_fetched = cache_timestamp.parse::<u64>().unwrap_or(0);
        let current_time = Self::current_time_in_seconds().unwrap_or(fetch_interval_seconds);

        current_time - last_fetched < fetch_interval_seconds
    }

    fn current_time_in_seconds() -> Result<u64, Error> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
    }

    fn cache_paths_for(&self, data_type: &str) -> Option<(PathBuf, PathBuf)> {
        self.cache_directory.as_ref().map(|directory| {
            (
                directory.join(format!("{data_type}.json")),
                directory.join(format!("{data_type}.last_fetched")),
            )
        })
    }
}

impl RedmineClient for RedmineHttpClient {
    fn get_all_activities(&self) -> anyhow::Result<Vec<TimeEntryActivity>> {
        let cache_paths = self.cache_paths_for("activities");

        if let Some(ref cache_paths) = cache_paths
            && !self.ignore_cache
            && cache_paths.0.exists()
            && cache_paths.1.exists()
            && Self::should_read_from_cache(&cache_paths.1, self.activities_fetch_interval_seconds)
        {
            // read from cache
            let cache_content = fs::read_to_string(&cache_paths.0)?;
            Ok(serde_json::from_str(&cache_content)?)
        } else {
            // fetch from server
            let endpoint = ListTimeEntryActivities::builder().build()?;
            let activities = self
                .redmine
                .json_response_body::<_, TimeEntryActivitiesWrapper<TimeEntryActivity>>(&endpoint)
                .map(|response| response.time_entry_activities)?;

            // write to cache
            if let Some((cache_path, cache_timestamp_path)) = cache_paths {
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let cache_content = serde_json::to_string(&activities)?;
                fs::write(&cache_path, &cache_content)?;
                fs::write(
                    &cache_timestamp_path,
                    format!("{}", Self::current_time_in_seconds()?),
                )?;
            }

            Ok(activities)
        }
    }

    fn get_all_projects(&self) -> anyhow::Result<Vec<Project>> {
        let cache_paths = self.cache_paths_for("projects");

        if let Some(ref cache_paths) = cache_paths
            && !self.ignore_cache
            && cache_paths.0.exists()
            && cache_paths.1.exists()
            && Self::should_read_from_cache(&cache_paths.1, self.projects_fetch_interval_seconds)
        {
            // read from cache
            let cache_content = fs::read_to_string(&cache_paths.0)?;
            Ok(serde_json::from_str(&cache_content)?)
        } else {
            // fetch from server
            let endpoint = ListProjects::builder()
                .status(vec![ProjectStatusFilter::Active])
                .build()?;
            let projects = self
                .redmine
                .json_response_body_all_pages::<_, Project>(&endpoint)?;

            // write to cache
            if let Some((cache_path, cache_timestamp_path)) = cache_paths {
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let cache_content = serde_json::to_string(&projects)?;
                fs::write(&cache_path, &cache_content)?;
                fs::write(
                    &cache_timestamp_path,
                    format!("{}", Self::current_time_in_seconds()?),
                )?;
            }

            Ok(projects)
        }
    }

    fn get_all_open_issues(&self) -> anyhow::Result<Vec<Issue>> {
        let cache_paths = self.cache_paths_for("issues");

        if let Some(ref cache_paths) = cache_paths
            && !self.ignore_cache
            && cache_paths.0.exists()
            && cache_paths.1.exists()
            && Self::should_read_from_cache(&cache_paths.1, self.issues_fetch_interval_seconds)
        {
            // read from cache
            let cache_content = fs::read_to_string(&cache_paths.0)?;
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
            if let Some((cache_path, cache_timestamp_path)) = cache_paths {
                if let Some(parent) = cache_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let cache_content = serde_json::to_string(&issues)?;
                fs::write(&cache_path, &cache_content)?;
                fs::write(
                    &cache_timestamp_path,
                    format!("{}", Self::current_time_in_seconds()?),
                )?;
            }

            Ok(issues)
        }
    }

    fn track_project_time(
        &self,
        activity_id: u64,
        project_id: u64,
        comment: &str,
        elapsed_hours: f64,
    ) -> anyhow::Result<()> {
        let endpoint = CreateTimeEntry::builder()
            .activity_id(activity_id)
            .project_id(project_id)
            .hours(elapsed_hours)
            .comments(comment.into())
            .build()?;
        self.redmine
            .json_response_body::<_, TimeEntryWrapper<TimeEntry>>(&endpoint)?;
        Ok(())
    }

    fn track_issue_time(
        &self,
        activity_id: u64,
        issue_id: u64,
        comment: &str,
        elapsed_hours: f64,
    ) -> anyhow::Result<()> {
        let endpoint = CreateTimeEntry::builder()
            .activity_id(activity_id)
            .issue_id(issue_id)
            .hours(elapsed_hours)
            .comments(comment.into())
            .build()?;
        self.redmine
            .json_response_body::<_, TimeEntryWrapper<TimeEntry>>(&endpoint)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MockRedmineClient, RedmineClient};
    use mockall::predicate::*;

    #[test]
    fn get_all_activities_returns_predefined_list() {
        let mut mock = MockRedmineClient::new();

        mock.expect_get_all_activities()
            .times(1)
            .returning(|| Ok(vec![]));

        let result = mock.get_all_activities();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn get_all_activities_propagates_error() {
        let mut mock = MockRedmineClient::new();

        mock.expect_get_all_activities()
            .times(1)
            .returning(|| Err(anyhow::anyhow!("network error")));

        assert!(mock.get_all_activities().is_err());
    }

    #[test]
    fn track_issue_time_called_with_correct_arguments() {
        let mut mock = MockRedmineClient::new();

        mock.expect_track_issue_time()
            .with(eq(7u64), eq(42u64), always(), eq(1.5f64))
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let comment = String::from("fixed it");
        let result = mock.track_issue_time(7, 42, &comment, 1.5);
        assert!(result.is_ok());
    }

    #[test]
    fn track_issue_time_returns_error_on_failure() {
        let mut mock = MockRedmineClient::new();

        mock.expect_track_issue_time()
            .times(1)
            .returning(|_, _, _, _| Err(anyhow::anyhow!("server rejected entry")));

        let result = mock.track_issue_time(1, 1, "", 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn track_project_time_called_with_correct_arguments() {
        let mut mock = MockRedmineClient::new();

        mock.expect_track_project_time()
            .with(eq(3u64), eq(99u64), always(), eq(2.0f64))
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let result = mock.track_project_time(3, 99, "", 2.0);
        assert!(result.is_ok());
    }

    #[test]
    fn unused_methods_are_never_called() {
        let mut mock = MockRedmineClient::new();

        mock.expect_get_all_projects().times(0);

        mock.expect_get_all_activities()
            .times(1)
            .returning(|| Ok(vec![]));

        mock.get_all_activities().unwrap();
    }
}
