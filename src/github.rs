use anyhow::{Context, Result, bail};
use chrono::{DateTime as ChronoDateTime, Utc};
use graphql_client::{Response, GraphQLQuery};
use log::{debug, error, info};

// GraphQL DateTime scalar type
type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug"
)]
pub struct UserActivity;

pub async fn fetch_activity(
    client: &reqwest::Client,
    username: &str,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
) -> Result<user_activity::ResponseData> {
    info!("Fetching activity from {} to {}", start_date, end_date);

    let variables = user_activity::Variables {
        username: username.to_string(),
        from: start_date.to_rfc3339(),
        to: end_date.to_rfc3339(),
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
