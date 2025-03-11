use chrono::{Duration, Utc};
use dotenv::dotenv;
use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use std::env;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/github.graphql",
    response_derives = "Debug"
)]
pub struct UserActivity;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let github_token = env::var("GITHUB_TOKEN")
        .expect("GITHUB_TOKEN environment variable is required");
    
    let username = "isserge";

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", github_token))?,
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str("github-activity-rs")?,
    );
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let activity = fetch_activity(&client, &username, Duration::weeks(1)).await?;
    println!("{}", serde_json::to_string_pretty(&activity)?);

    Ok(())
}

async fn fetch_activity(
    client: &reqwest::Client,
    username: &str,
    duration: Duration,
) -> Result<Value, Box<dyn std::error::Error>> {
    let end_date = Utc::now();
    let _start_date = end_date - duration;

    let variables = user_activity::Variables {
        username: username.to_string(),
        // from: start_date.to_rfc3339(),
        // to: end_date.to_rfc3339(),
    };

    let request_body = UserActivity::build_query(variables);

    let res = client
        .post("https://api.github.com/graphql")
        .json(&request_body)
        .send()
        .await?;

    // Print the raw response for debugging
    let text = res.text().await?;
    println!("Raw response: {}", text);
    
    let response_body: Response<Value> = serde_json::from_str(&text)?;
    
    if let Some(errors) = response_body.errors {
        eprintln!("GraphQL Errors: {:?}", errors);
    }

    Ok(response_body.data.unwrap_or_default())
}
