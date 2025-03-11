use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use clap::Parser;
use dotenv::dotenv;
use graphql_client::{GraphQLQuery, Response};
use log::{debug, error, info};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use std::env;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// GitHub username
    #[arg(short, long)]
    username: String,

    /// Time period (day, week, month)
    #[arg(short, long, value_parser = parse_period)]
    period: Duration,
}

fn parse_period(arg: &str) -> Result<Duration, String> {
    match arg.to_lowercase().as_str() {
        "day" => Ok(Duration::days(1)),
        "week" => Ok(Duration::weeks(1)),
        "month" => Ok(Duration::days(30)),
        _ => Err(format!(
            "Invalid period: {}. Use 'day', 'week', or 'month'",
            arg
        )),
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/github.graphql",
    response_derives = "Debug, Default, serde::Serialize",
    variables_derives = "Debug"
)]
pub struct UserActivity;

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

    let activity = fetch_activity(&client, &args.username, args.period).await?;
    info!("Activity fetched successfully.");

    println!(
        "{}",
        serde_json::to_string_pretty(&activity).context("Failed to serialize activity to JSON")?
    );

    Ok(())
}

async fn fetch_activity(
    client: &reqwest::Client,
    username: &str,
    duration: Duration,
) -> Result<user_activity::ResponseData> {
    let end_date = Utc::now();
    let start_date = end_date - duration;
    info!("Fetching activity from {} to {}", start_date, end_date);

    let variables = user_activity::Variables {
        username: username.to_string(),
    };

    let request_body = UserActivity::build_query(variables);
    debug!("GraphQL request body: {:?}", request_body);

    let res = client
        .post("https://api.github.com/graphql")
        .json(&request_body)
        .send()
        .await
        .context("Failed to send request to GitHub GraphQL API")?;
    info!("Request sent, awaiting response.");

    let response_body: Response<user_activity::ResponseData> = res
        .json()
        .await
        .context("Failed to parse JSON response from GitHub GraphQL API")?;
    debug!("GraphQL response: {:?}", response_body);

    if let Some(errors) = response_body.errors {
        error!("GraphQL errors: {:?}", errors);
        bail!("GraphQL errors: {:?}", errors);
    }

    response_body
        .data
        .ok_or_else(|| anyhow::anyhow!("No data received in GraphQL response"))
}
