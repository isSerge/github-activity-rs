#![warn(missing_docs)]
//! GitHub Activity Reporter: a command-line tool that fetches and formats GitHub activity.

mod args;
mod filter;
mod format;
mod github;

use anyhow::Context;
use args::{Args, OutputFormat};
use clap::Parser;
use dotenv::dotenv;
use format::{format_markdown, format_plain};
use log::{debug, info};
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    if let Err(err) = run().await {
        eprintln!("Error: {}", format_error(&err));
        std::process::exit(1);
    }
}

/// Run the core logic of the program.
async fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    info!("Starting GitHub activity fetch for user: {}", args.username);

    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable is required")?;
    debug!("GitHub token retrieved successfully.");

    let (start_date, end_date) = args
        .get_date_range()
        .map_err(|e| anyhow::anyhow!("Failed to get date range: {}", e))?;
    info!("Fetching activity from {} to {}", start_date, end_date);

    let github_client = github::GithubClient::new(
        github_token,
        args.username.to_string(),
        start_date,
        end_date,
    )
    .context("Failed to create GitHub client")?;

    let activity = github_client
        .fetch_activity()
        .await
        .context("Failed to fetch activity from GitHub API")?;
    info!("Activity fetched successfully.");

    let filtered_activity = filter::filter_activity(activity, &args.repo, &args.org);

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

/// Format an error message for the user.
fn format_error(error: &anyhow::Error) -> String {
    // Check if the error is a reqwest error and further, what kind it is.
    if let Some(reqwest_err) = error.downcast_ref::<reqwest::Error>() {
        if reqwest_err.is_connect() {
            return format!("Network connection error: {}", reqwest_err);
        } else if reqwest_err.is_timeout() {
            return format!("Network timeout error: {}", reqwest_err);
        } else {
            return format!("HTTP error: {}", reqwest_err);
        }
    }
    // Check if the error came from JSON parsing.
    if let Some(json_err) = error.downcast_ref::<serde_json::Error>() {
        return format!("Data parsing error: {}", json_err);
    }
    // Fallback to showing the full error chain.
    format!("{:#}", error)
}
