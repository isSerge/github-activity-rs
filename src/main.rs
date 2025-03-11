use chrono::{Duration, Utc};
use clap::Parser;
use dotenv::dotenv;
use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
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
        _ => Err(format!("Invalid period: {}. Use 'day', 'week', or 'month'", arg)),
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/github.graphql",
    response_derives = "Debug, Default, serde::Serialize"
)]
pub struct UserActivity;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let args = Args::parse();
    
    let github_token = env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN environment variable is required");

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", github_token))?,
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));
    
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let activity = fetch_activity(&client, &args.username, args.period).await?;
    println!("{}", serde_json::to_string_pretty(&activity)?);

    Ok(())
}

async fn fetch_activity(
    client: &reqwest::Client,
    username: &str,
    duration: Duration,
) -> Result<user_activity::ResponseData, Box<dyn std::error::Error>> {
    let end_date = Utc::now();
    let _start_date = end_date - duration;

    let variables = user_activity::Variables {
        username: username.to_string(),
    };

    let request_body = UserActivity::build_query(variables);

    let res = client
        .post("https://api.github.com/graphql")
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<user_activity::ResponseData> = res.json().await?;
    
    if let Some(errors) = response_body.errors {
        eprintln!("GraphQL Errors: {:?}", errors);
    }

    Ok(response_body.data.unwrap_or_default())
}
