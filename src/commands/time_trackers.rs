// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::models::cli::TimeTrackCommands;
use crate::models::configuration::Configuration;
use crate::models::skim::{
    RedmineActivityItem, RedmineProjectIssueItem, RedmineProjectIssueItemVariant,
    create_skim_options,
};
use crate::models::time_entry::StartTime;
use crate::redmine::client::RedmineClient;
use redmine_api::api::enumerations::TimeEntryActivity;
use redmine_api::api::issues::Issue;
use redmine_api::api::projects::Project;
use skim::Skim;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn dispatch(command: &TimeTrackCommands, configuration: &Configuration) -> anyhow::Result<()> {
    match command {
        TimeTrackCommands::Start(args) => start(
            configuration,
            args.server_selection.server.as_ref(),
            args.activity.as_ref(),
            args.project.as_ref(),
            args.issue.as_ref(),
            args.comment.as_ref(),
            args.ignore_cache,
        ),
        TimeTrackCommands::Stop(args) => {
            stop(configuration, args.server_selection.server.as_ref(), None)
        }
        TimeTrackCommands::Show(args) => show(
            configuration,
            args.server_selection.server.as_ref(),
            args.format.as_ref(),
        ),
    }
}

type IdWithText = (u64, String);

fn start(
    configuration: &Configuration,
    server_name: Option<&String>,
    activity: Option<&String>,
    project: Option<&String>,
    issue: Option<&String>,
    comment: Option<&String>,
    ignore_cache: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_server(server_name) {
        let redmine: Box<dyn RedmineClient> =
            configuration.create_redmine_client(registration, ignore_cache)?;
        let tracking_file_path = configuration.tracking_file_path(&registration.name)?;

        if tracking_file_path.exists() {
            stop(configuration, server_name, Some(redmine.as_ref()))?;
        }

        let time_entry_activities = redmine.get_all_activities()?;
        let activity_id = determine_activity_id(activity, &time_entry_activities);

        if activity.is_some() && activity_id.is_none() {
            anyhow::bail!("Specified activity not known in server");
        }

        if let Some(activity_tuple) = activity_id {
            let (project_tuple, issue_tuple): (Option<IdWithText>, Option<IdWithText>) =
                match (project, issue) {
                    (Some(project), _) => {
                        // user has specified a project
                        let projects = redmine.get_all_projects()?;
                        let tuple = determine_project(project, &projects);
                        if tuple.0.is_none() {
                            anyhow::bail!("Specified project not known in server");
                        }
                        tuple
                    }
                    (_, Some(issue)) => {
                        // user has specified an issue
                        let issues = redmine.get_all_open_issues()?;
                        let tuple = determine_issue(issue, &issues);
                        if tuple.1.is_none() {
                            anyhow::bail!("Specified issue not known in server");
                        }
                        tuple
                    }
                    (None, None) => {
                        // show both projects && issues and allow fuzzy-select
                        let projects = redmine.get_all_projects()?;
                        let issues = redmine.get_all_open_issues()?;
                        let items = combine_projects_and_issues(&projects, &issues);

                        Skim::run_items(
                            create_skim_options("Select the project/issue you want to work on:"),
                            items,
                        )
                        .ok()
                        .filter(|output| !output.is_abort)
                        .and_then(|output| output.selected_items.first().cloned())
                        .map_or((None, None), |first| {
                            first
                                .downcast_item::<RedmineProjectIssueItem>()
                                .map_or((None, None), map_selection)
                        })
                    }
                };

            match (project_tuple, issue_tuple) {
                (None, None) => {
                    anyhow::bail!("No project/issue selected. No time will be tracked");
                }
                (project, issue) => {
                    if let Some(parent) = tracking_file_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    write_tracking_file(
                        tracking_file_path,
                        &StartTime {
                            server: registration.name.clone(),
                            activity_id: activity_tuple.0,
                            activity: activity_tuple.1,
                            project_id: project.clone().map(|project| project.0),
                            project: project.map(|project| project.1),
                            issue_id: issue.clone().map(|issue| issue.0),
                            issue: issue.map(|issue| issue.1),
                            comment: comment.cloned(),
                            start: chrono::Utc::now(),
                        },
                    )?;
                }
            }

            Ok(())
        } else {
            anyhow::bail!("No redmine activity found")
        }
    } else {
        anyhow::bail!(
            "No server found in configuration. Run 'redclock server add ...' first to add one"
        )
    }
}

fn map_selection(selection: &RedmineProjectIssueItem) -> (Option<IdWithText>, Option<IdWithText>) {
    if selection.variant == RedmineProjectIssueItemVariant::Project {
        (
            Some((
                selection.project_id.unwrap_or_default(),
                selection.project_name.clone().unwrap_or_default(),
            )),
            None,
        )
    } else {
        (
            None,
            Some((
                selection.issue_id.unwrap_or_default(),
                selection.issue_title.clone().unwrap_or_default(),
            )),
        )
    }
}

fn determine_issue(issue: &str, issues: &[Issue]) -> (Option<IdWithText>, Option<IdWithText>) {
    issue.trim().parse::<u64>().map_or_else(
        |_| {
            (
                None,
                issues
                    .iter()
                    .find(|&entry| {
                        entry
                            .subject
                            .as_ref()
                            .filter(|&subject| subject.eq_ignore_ascii_case(issue))
                            .is_some()
                    })
                    .map(|entry| (entry.id, entry.subject.clone().unwrap_or_default()))
                    .or(None),
            )
        },
        |issue_id| {
            (
                None,
                issues
                    .iter()
                    .find(|entry| entry.id == issue_id)
                    .map(|entry| (issue_id, entry.subject.clone().unwrap_or_default()))
                    .or(None),
            )
        },
    )
}

fn determine_project(
    project: &str,
    projects: &[Project],
) -> (Option<IdWithText>, Option<IdWithText>) {
    project.trim().parse::<u64>().map_or_else(
        |_| {
            (
                projects
                    .iter()
                    .find(|&entry| entry.name.eq_ignore_ascii_case(project))
                    .map(|entry| (entry.id, entry.name.clone()))
                    .or(None),
                None,
            )
        },
        |project_id| {
            (
                projects
                    .iter()
                    .find(|&entry| entry.id == project_id)
                    .map(|entry| (project_id, entry.name.clone()))
                    .or(None),
                None,
            )
        },
    )
}

fn combine_projects_and_issues(
    projects: &[Project],
    issues: &[Issue],
) -> Vec<RedmineProjectIssueItem> {
    let mut items: Vec<RedmineProjectIssueItem> = projects
        .iter()
        .map(|project| RedmineProjectIssueItem {
            project_id: Some(project.id),
            project_name: Some(project.name.clone()),
            project_identifier: Some(project.identifier.clone()),
            issue_id: None,
            issue_title: None,
            variant: RedmineProjectIssueItemVariant::Project,
        })
        .collect();
    for issue in issues {
        items.push(RedmineProjectIssueItem {
            project_id: None,
            project_name: None,
            project_identifier: None,
            issue_id: Some(issue.id),
            issue_title: Some(issue.subject.clone().unwrap_or_default()),
            variant: RedmineProjectIssueItemVariant::Issue,
        });
    }
    items
}

fn write_tracking_file(tracking_file_path: PathBuf, start_time: &StartTime) -> anyhow::Result<()> {
    let mut tracking_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(tracking_file_path)?;
    let t = toml::to_string(&start_time)?;
    let toml = t.as_bytes();
    tracking_file.write_all(toml)?;
    tracking_file.flush()?;
    Ok(())
}

fn determine_activity_id(
    activity: Option<&String>,
    time_entry_activities: &[TimeEntryActivity],
) -> Option<IdWithText> {
    activity.map_or_else(
        || {
            Skim::run_items(
                create_skim_options("Select the activity you wish to start:"),
                time_entry_activities
                    .iter()
                    .map(|entry| RedmineActivityItem {
                        activity_id: entry.id,
                        activity_name: entry.name.clone(),
                    }),
            )
            .ok()
            .filter(|output| !output.is_abort)
            .and_then(|output| output.selected_items.first().cloned())
            .and_then(|first| {
                first
                    .downcast_item::<RedmineActivityItem>()
                    .map(|item| (item.activity_id, item.activity_name.clone()))
            })
        },
        |activity| {
            activity.trim().parse::<u64>().map_or_else(
                |_| {
                    time_entry_activities
                        .iter()
                        .filter(|&entry| entry.name.eq_ignore_ascii_case(activity))
                        .map(|entry| (entry.id, entry.name.clone()))
                        .next()
                },
                |activity_id| {
                    time_entry_activities
                        .iter()
                        .find(|entry| entry.id == activity_id)
                        .map(|entry| (activity_id, entry.name.clone()))
                        .or(None)
                },
            )
        },
    )
}

fn stop(
    configuration: &Configuration,
    server_name: Option<&String>,
    redmine_client: Option<&dyn RedmineClient>,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_server(server_name) {
        let tracking_file_path = configuration.tracking_file_path(&registration.name)?;

        if tracking_file_path.exists() {
            // Either use the passed-in client or create a fresh one (boxed as trait object).
            let owned: Box<dyn RedmineClient>;
            let redmine: &dyn RedmineClient = if let Some(client) = redmine_client {
                client
            } else {
                owned = configuration.create_redmine_client(registration, false)?;
                owned.as_ref()
            };

            let tracking_file_content = fs::read_to_string(&tracking_file_path)?;
            let start_time: StartTime = toml::from_str(&tracking_file_content)?;
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(start_time.start);
            let duration = elapsed.to_std()?;
            let elapsed_hours = (duration.as_secs_f64() / 60f64) / 60f64;

            match (start_time.project_id, start_time.issue_id) {
                (Some(project_id), None) => {
                    redmine.track_project_time(
                        start_time.activity_id,
                        project_id,
                        start_time.comment.as_deref().unwrap_or_default(),
                        elapsed_hours,
                    )?;
                }
                (None, Some(issue_id)) => {
                    redmine.track_issue_time(
                        start_time.activity_id,
                        issue_id,
                        start_time.comment.as_deref().unwrap_or_default(),
                        elapsed_hours,
                    )?;
                }
                (_, _) => {}
            }

            Ok(fs::remove_file(tracking_file_path)?)
        } else {
            Ok(())
        }
    } else {
        anyhow::bail!(
            "No server found in configuration. Run 'redclock server add ...' first to add one"
        )
    }
}

#[allow(clippy::literal_string_with_formatting_args)]
fn show(
    configuration: &Configuration,
    server_name: Option<&String>,
    format: Option<&String>,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_server(server_name) {
        let tracking_file_path = configuration.tracking_file_path(&registration.name)?;

        if tracking_file_path.exists() {
            let tracking_file_content = fs::read_to_string(&tracking_file_path)?;
            let start_time: StartTime = toml::from_str(&tracking_file_content)?;
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(start_time.start);
            let duration = elapsed.to_std()?;
            let seconds = duration.as_secs() % 60;
            let minutes = (duration.as_secs() / 60) % 60;
            let hours = (duration.as_secs() / 60) / 60;
            let format_to_use = format
                .cloned()
                .unwrap_or_else(|| String::from("{project}{issue} for {hours}:{minutes}"));
            let output = format_to_use
                .replace("{activity}", &start_time.activity)
                .replace("{project}", &start_time.project.unwrap_or_default())
                .replace("{issue}", &start_time.issue.unwrap_or_default())
                .replace("{hours}", &format!("{hours:0>2}"))
                .replace("{minutes}", &format!("{minutes:0>2}"))
                .replace("{seconds}", &format!("{seconds:0>2}"));
            println!("{output}");
        }
        Ok(())
    } else {
        anyhow::bail!(
            "No server found in configuration. Run 'redclock server add ...' first to add one"
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::configuration::ServerRegistration;
    use crate::redmine::client::MockRedmineClient;
    use mockall::predicate::*;
    use redmine_api::api::issues::Issue;
    use redmine_api::api::projects::Project;

    fn make_test_configuration(dir: &tempfile::TempDir) -> Configuration {
        Configuration {
            servers: vec![ServerRegistration {
                name: "test-server".to_string(),
                url: "https://redmine.example.com".to_string(),
                api_key: Some("test-api-key".to_string()),
                api_key_command: None,
            }],
            default_server: Some("test-server".to_string()),
            data_dir_override: Some(dir.path().to_path_buf()),
            ..Default::default()
        }
    }

    fn make_activity(id: u64, name: &str) -> TimeEntryActivity {
        TimeEntryActivity {
            id,
            name: name.to_string(),
            is_default: false,
            active: false,
        }
    }

    fn make_project(id: u64, name: &str) -> Project {
        let json = serde_json::json!({
            "id": id,
            "name": name,
            "identifier": name.to_lowercase(),
            "description": null,
            "is_public": null,
            "inherit_members": null,
            "status": 1,
            "created_on": "2024-01-01T00:00:00Z",
            "updated_on": "2024-01-01T00:00:00Z"
        });
        serde_json::from_value(json).expect("failed to deserialize test Project")
    }

    fn make_issue(id: u64, subject: &str) -> Issue {
        let json = serde_json::json!({
            "id": id,
            "subject": subject,
            "project": { "id": 1, "name": "Test Project" },
            "tracker": { "id": 1, "name": "Bug" },
            "status": { "id": 1, "name": "New", "is_closed": false },
            "priority": { "id": 1, "name": "Normal" },
            "author": { "id": 1, "name": "Test User" },
            "description": null,
            "done_ratio": 0,
            "created_on": "2024-01-01T00:00:00Z",
            "updated_on": "2024-01-01T00:00:00Z",
            "closed_on": "2024-01-01T00:00:00Z"
        });
        serde_json::from_value(json).expect("failed to deserialize test Issue")
    }

    #[test]
    fn activity_found_by_name_case_insensitive() {
        let activities = vec![make_activity(3, "Development")];
        let name = String::from("development");
        let result = determine_activity_id(Some(&name), &activities);
        assert_eq!(result, Some((3, "Development".to_string())));
    }

    #[test]
    fn activity_found_by_numeric_id() {
        let activities = vec![make_activity(7, "Testing")];
        let id = String::from("7");
        let result = determine_activity_id(Some(&id), &activities);
        assert_eq!(result, Some((7, "Testing".to_string())));
    }

    #[test]
    fn activity_not_found_returns_none() {
        let activities = vec![make_activity(1, "Design")];
        let name = String::from("nonexistent");
        let result = determine_activity_id(Some(&name), &activities);
        assert!(result.is_none());
    }

    #[test]
    fn project_found_by_name() {
        let projects = vec![make_project(10, "Alpha")];
        let (proj, issue) = determine_project("alpha", &projects);
        assert_eq!(proj, Some((10, "Alpha".to_string())));
        assert!(issue.is_none());
    }

    #[test]
    fn project_found_by_id() {
        let projects = vec![make_project(10, "Alpha")];
        let (proj, issue) = determine_project("10", &projects);
        assert_eq!(proj, Some((10, "Alpha".to_string())));
        assert!(issue.is_none());
    }

    #[test]
    fn project_not_found_returns_none() {
        let projects = vec![make_project(10, "Alpha")];
        let (proj, issue) = determine_project("Beta", &projects);
        assert!(proj.is_none());
        assert!(issue.is_none());
    }

    #[test]
    fn issue_found_by_subject() {
        let issues = vec![make_issue(42, "Fix login bug")];
        let (proj, iss) = determine_issue("fix login bug", &issues);
        assert!(proj.is_none());
        assert_eq!(iss, Some((42, "Fix login bug".to_string())));
    }

    #[test]
    fn issue_found_by_id() {
        let issues = vec![make_issue(42, "Fix login bug")];
        let (proj, iss) = determine_issue("42", &issues);
        assert!(proj.is_none());
        assert_eq!(iss, Some((42, "Fix login bug".to_string())));
    }

    #[test]
    fn issue_not_found_returns_none() {
        let issues = vec![make_issue(1, "Some issue")];
        let (proj, iss) = determine_issue("999", &issues);
        assert!(proj.is_none());
        assert!(iss.is_none());
    }

    #[test]
    fn stop_calls_track_project_time_when_tracking_file_has_project() {
        // Write a fake tracking file so stop() has something to read.
        let dir = tempfile::tempdir().unwrap();
        let configuration = make_test_configuration(&dir);
        let registration = configuration.servers.first().unwrap();
        let start_time = &StartTime {
            server: registration.url.clone(),
            activity_id: 7u64,
            activity: String::from("Development"),
            project_id: Some(5u64),
            project: Some(String::from("Test")),
            issue_id: None,
            issue: None,
            comment: None,
            start: chrono::Utc::now(),
        };
        let tracking_file_path = configuration
            .tracking_file_path(&registration.name.clone())
            .unwrap();
        if let Some(parent) = tracking_file_path.parent() {
            fs::create_dir_all(parent).expect("failed to create tracking file")
        }
        write_tracking_file(tracking_file_path, start_time).unwrap();

        let mut mock = MockRedmineClient::new();
        mock.expect_track_project_time()
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let result = stop(&configuration, None, Some(&mock));
        assert!(result.is_ok());
    }

    #[test]
    fn stop_calls_track_issue_time_when_tracking_file_has_issue() {
        let dir = tempfile::tempdir().unwrap();
        let configuration = make_test_configuration(&dir);
        let registration = configuration.servers.first().unwrap();
        let start_time = &StartTime {
            server: registration.url.clone(),
            activity_id: 7u64,
            activity: String::from("Development"),
            project_id: None,
            project: None,
            issue_id: Some(3u64),
            issue: Some(String::from("Some Test Issue")),
            comment: None,
            start: chrono::Utc::now(),
        };
        let tracking_file_path = configuration
            .tracking_file_path(&registration.name.clone())
            .unwrap();
        if let Some(parent) = tracking_file_path.parent() {
            fs::create_dir_all(parent).expect("failed to create tracking file");
        }
        write_tracking_file(tracking_file_path, start_time).unwrap();

        let mut mock = MockRedmineClient::new();
        mock.expect_track_issue_time()
            .times(1)
            .returning(|_, _, _, _| Ok(()));

        let result = stop(&configuration, None, Some(&mock));
        assert!(result.is_ok());
    }

    #[test]
    fn stop_propagates_tracking_error() {
        let dir = tempfile::tempdir().unwrap();
        let configuration = make_test_configuration(&dir);
        let registration = configuration.servers.first().unwrap();
        let start_time = &StartTime {
            server: registration.url.clone(),
            activity_id: 7u64,
            activity: String::from("Development"),
            project_id: Some(3u64),
            project: Some(String::from("Test")),
            issue_id: None,
            issue: None,
            comment: None,
            start: chrono::Utc::now(),
        };
        let tracking_file_path = configuration
            .tracking_file_path(&registration.name.clone())
            .unwrap();
        if let Some(parent) = tracking_file_path.parent() {
            fs::create_dir_all(parent).expect("failed to create tracking file");
        }
        write_tracking_file(tracking_file_path, start_time).unwrap();

        let mut mock = MockRedmineClient::new();
        mock.expect_track_project_time()
            .times(1)
            .returning(|_, _, _, _| Err(anyhow::anyhow!("server rejected")));

        let result = stop(&configuration, None, Some(&mock));
        assert!(result.is_err());
    }
}
