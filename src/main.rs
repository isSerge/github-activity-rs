mod args;
mod github;

use clap::Parser;
use anyhow::{Context, Result};
use args::Args;
use dotenv::dotenv;
use log::{debug, info};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let (start_date, end_date) = args.get_date_range()
        .map_err(|e| anyhow::anyhow!("Failed to get date range: {}", e))?;
    info!("Fetching activity from {} to {}", start_date, end_date);

    let activity = github::fetch_activity(&client, &args.username.to_string(), start_date, end_date).await?;
    info!("Activity fetched successfully.");

    println!(
        "{}",
        serde_json::to_string_pretty(&activity).context("Failed to serialize activity to JSON")?
    );

    Ok(())
}
