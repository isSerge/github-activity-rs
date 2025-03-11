mod args;
mod format;
mod github;

use crate::github::user_activity;
use anyhow::Context;
use args::{Args, OutputFormat};
use clap::Parser;
use dotenv::dotenv;
use format::{format_markdown, format_plain};
use log::{debug, info};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use std::env;

// Helper function to filter activity based on repo and organization filters.
fn filter_activity(
    mut activity: user_activity::ResponseData,
    repo_filter: &Option<String>,
    org_filter: &Option<String>,
) -> user_activity::ResponseData {
    if let Some(user) = activity.user.as_mut() {
        // Access contributions_collection field.
        let cc = &user.contributions_collection;
        // We clone the current list and filter it, then replace the list.
        let mut filtered_repos = cc.commit_contributions_by_repository.clone();

        if let Some(repo_filter) = repo_filter {
            filtered_repos.retain(|repo_contrib| {
                // Field names should be in snake_case per graphql_client conversion.
                repo_contrib.repository.name_with_owner == *repo_filter
            });
        }

        if let Some(org_filter) = org_filter {
            filtered_repos.retain(|repo_contrib| {
                repo_contrib
                    .repository
                    .name_with_owner
                    .starts_with(&format!("{}/", org_filter))
            });
        }

        // If needed, update the user’s contributions_collection with the filtered list.
        // (This requires that contributions_collection is mutable. Since we're cloning
        // the user in the ResponseData, we can build a new ResponseData with the filtered list.)
        if let Some(mut user) = activity.user {
            user.contributions_collection
                .commit_contributions_by_repository = filtered_repos;
            activity.user = Some(user);
        }
    }
    activity
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();

    let args = Args::parse();
    info!("Starting GitHub activity fetch for user: {}", args.username);

    let github_token =
        env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN environment variable is required");
    debug!("GitHub token retrieved successfully.");

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", github_token))?,
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    debug!("HTTP client built successfully.");

    let (start_date, end_date) = args
        .get_date_range()
        .map_err(|e| anyhow::anyhow!("Failed to get date range: {}", e))?;
    info!("Fetching activity from {} to {}", start_date, end_date);

    let activity =
        github::fetch_activity(&client, &args.username.to_string(), start_date, end_date).await?;
    info!("Activity fetched successfully.");

    let filtered_activity = filter_activity(activity, &args.repo, &args.org);

    match args.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&filtered_activity)
                    .context("Failed to serialize activity to JSON")?
            );
        }
        OutputFormat::Plain => {
            println!(
                "{}",
                format_plain(&filtered_activity, start_date, end_date, &args.username.0)
            );
        }
        OutputFormat::Markdown => {
            println!(
                "{}",
                format_markdown(&filtered_activity, start_date, end_date, &args.username.0)
            );
        }
    }

    Ok(())
}
