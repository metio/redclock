// SPDX-FileCopyrightText: The redclock Authors
// SPDX-License-Identifier: 0BSD

use crate::models::cli::TimeTrackCommands;
use crate::models::configuration::Configuration;
use crate::models::skim::{
    RedmineActivityItem, RedmineProjectIssueItem, RedmineProjectIssueItemVariant,
    create_skim_options,
};
use crate::models::time_entry::StartTime;
use crate::redmine::client::{RedmineClient, RedmineHttpClient};
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
            args.ignore_cache
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
        let redmine = configuration.create_redmine_client(registration, ignore_cache)?;
        let tracking_file_path = Configuration::tracking_file_path(&registration.name)?;

        if tracking_file_path.exists() {
            stop(configuration, server_name, Some(&redmine))?;
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
                            first.downcast_item::<RedmineProjectIssueItem>().map_or(
                                (None, None),
                                |selection| {
                                    if selection.variant == RedmineProjectIssueItemVariant::Project
                                    {
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
                                },
                            )
                        })
                    }
                };

            if let Some(parent) = &tracking_file_path.parent() {
                fs::create_dir_all(parent)?;
            }

            write_tracking_file(
                tracking_file_path,
                &StartTime {
                    server: registration.name.clone(),
                    activity_id: activity_tuple.0,
                    activity: activity_tuple.1,
                    project_id: project_tuple.clone().map(|project| project.0),
                    project: project_tuple.map(|project| project.1),
                    issue_id: issue_tuple.clone().map(|issue| issue.0),
                    issue: issue_tuple.map(|issue| issue.1),
                    comment: comment.cloned(),
                    start: chrono::Utc::now(),
                },
            )?;

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
                            .filter(|&subject| {
                                subject.eq_ignore_ascii_case(issue)
                            })
                            .is_some()
                    })
                    .map(|entry| {
                        (entry.id, entry.subject.clone().unwrap_or_default())
                    })
                    .or(None),
            )
        },
        |issue_id| {
            (
                None,
                issues
                    .iter()
                    .find(|entry| entry.id == issue_id)
                    .map(|entry| {
                        (issue_id, entry.subject.clone().unwrap_or_default())
                    })
                    .or(None),
            )
        },
    )
}

fn determine_project(project: &str, projects: &[Project]) -> (Option<IdWithText>, Option<IdWithText>) {
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
    redmine_client: Option<&RedmineHttpClient>,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_server(server_name) {
        let tracking_file_path = Configuration::tracking_file_path(&registration.name)?;

        if tracking_file_path.exists() {
            let redmine = if let Some(client) = redmine_client {
                client
            } else {
                &configuration.create_redmine_client(registration, false)?
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
                        start_time.comment.as_ref(),
                        elapsed_hours,
                    )?;
                }
                (None, Some(issue_id)) => {
                    redmine.track_issue_time(
                        start_time.activity_id,
                        issue_id,
                        start_time.comment.as_ref(),
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
fn show(configuration: &Configuration, server_name: Option<&String>, format: Option<&String>) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_server(server_name) {
        let tracking_file_path = Configuration::tracking_file_path(&registration.name)?;

        if tracking_file_path.exists() {
            let tracking_file_content = fs::read_to_string(&tracking_file_path)?;
            let start_time: StartTime = toml::from_str(&tracking_file_content)?;
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(start_time.start);
            let duration = elapsed.to_std()?;
            let seconds = duration.as_secs() % 60;
            let minutes = (duration.as_secs() / 60) % 60;
            let hours = (duration.as_secs() / 60) / 60;
            let format_to_use = format.cloned().unwrap_or_else(|| String::from("{project}{issue} for {hours}:{minutes}"));
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
