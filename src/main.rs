mod args;
mod github;
mod format;

use format::{format_plain, format_markdown};
use anyhow::Context;
use args::{Args, OutputFormat};
use clap::Parser;
use dotenv::dotenv;
use log::{debug, info};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use std::env;

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

    // Assuming the generated type has a structure like:
    // activity.user.contributionsCollection.commitContributionsByRepository
    let mut filtered_activity = activity;

    // If a repo filter is provided, retain only contributions from that repository.
    if let Some(ref repo_filter) = args.repo {
        if let Some(ref mut contributions) = filtered_activity
            .user
            .as_mut()
            .and_then(|u| Some(&mut u.contributions_collection))
        {
            contributions
                .commit_contributions_by_repository
                .retain(|repo_contrib| repo_contrib.repository.name_with_owner == *repo_filter);
        }
    }

    // If an organization filter is provided, retain only contributions from repos within that organization.
    if let Some(ref org_filter) = args.org {
        if let Some(ref mut contributions) = filtered_activity
            .user
            .as_mut()
            .and_then(|u| Some(&mut u.contributions_collection))
        {
            contributions
                .commit_contributions_by_repository
                .retain(|repo_contrib| {
                    // Check if the repository name starts with "org_filter/".
                    repo_contrib
                        .repository
                        .name_with_owner
                        .starts_with(&format!("{}/", org_filter))
                });
        }
    }

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
