use anyhow::{Context, Result, bail};
use chrono::{DateTime as ChronoDateTime, Utc};
use graphql_client::{GraphQLQuery, Response};
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

/// Fetches user activity with pagination for issues, pull requests, and PR reviews.
pub async fn fetch_activity(
    client: &reqwest::Client,
    username: &str,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
) -> Result<user_activity::ResponseData> {
    // Set the page size for paginated connections.
    let issues_first = 10;
    let prs_first = 10;
    let pr_reviews_first = 10;

    // Initialize pagination cursors as None.
    let mut issues_after: Option<String> = None;
    let mut prs_after: Option<String> = None;
    let mut pr_reviews_after: Option<String> = None;

    // Accumulators for nodes.
    let mut all_issue_nodes = Vec::new();
    let mut all_pr_nodes = Vec::new();
    let mut all_pr_review_nodes = Vec::new();

    // We'll perform the query repeatedly until no connection has a next page.
    // For non-paginated fields, we capture them once.
    let mut base_cc = None;

    loop {
        let variables = user_activity::Variables {
            username: username.to_string(),
            from: start_date.to_rfc3339(),
            to: end_date.to_rfc3339(),
            issues_first,
            issues_after: issues_after.clone(),
            prs_first,
            prs_after: prs_after.clone(),
            pr_reviews_first,
            pr_reviews_after: pr_reviews_after.clone(),
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

        let data = response_body
            .data
            .ok_or_else(|| anyhow::anyhow!("No data received in GraphQL response"))?;

        // Capture the contributionsCollection once.
        if base_cc.is_none() {
            // Assuming user is always Some.
            base_cc = Some(data.user.clone().unwrap().contributions_collection.clone());
        }

        // Merge Issue Contributions.
        {
            let user = data.user.clone().unwrap();
            let issue_conn = user.contributions_collection.issue_contributions;
            if let Some(nodes) = issue_conn.nodes {
                all_issue_nodes.extend(nodes);
            }
            // page_info is a struct (not Option).
            let page_info = issue_conn.page_info;
            if page_info.has_next_page {
                issues_after = page_info.end_cursor;
            } else {
                issues_after = None;
            }
        }

        // Merge Pull Request Contributions.
        {
            let user = data.user.clone().unwrap();
            let pr_conn = user.contributions_collection.pull_request_contributions;
            if let Some(nodes) = pr_conn.nodes {
                all_pr_nodes.extend(nodes);
            }
            let page_info = pr_conn.page_info;
            if page_info.has_next_page {
                prs_after = page_info.end_cursor;
            } else {
                prs_after = None;
            }
        }

        // Merge Pull Request Review Contributions.
        {
            let user = data.user.clone().unwrap();
            let pr_review_conn = user
                .contributions_collection
                .pull_request_review_contributions;
            if let Some(nodes) = pr_review_conn.nodes {
                all_pr_review_nodes.extend(nodes);
            }
            let page_info = pr_review_conn.page_info;
            if page_info.has_next_page {
                pr_reviews_after = page_info.end_cursor;
            } else {
                pr_reviews_after = None;
            }
        }

        // If none of the connections have a next page, break out of the loop.
        if issues_after.is_none() && prs_after.is_none() && pr_reviews_after.is_none() {
            // Build final ResponseData by replacing the connection nodes with the accumulated ones.
            let mut final_data = data;
            {
                // Extract the user, update its contributionsCollection, then reassign.
                let mut user = final_data.user.unwrap();
                user.contributions_collection.issue_contributions.nodes = Some(all_issue_nodes);
                user.contributions_collection
                    .pull_request_contributions
                    .nodes = Some(all_pr_nodes);
                user.contributions_collection
                    .pull_request_review_contributions
                    .nodes = Some(all_pr_review_nodes);
                // Restore non-paginated contributionsCollection from base_cc.
                user.contributions_collection = base_cc.unwrap();
                final_data.user = Some(user);
            }
            return Ok(final_data);
        }
    }
}
