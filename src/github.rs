use anyhow::{Context, Result, bail};
use chrono::{DateTime as ChronoDateTime, Utc};
use futures::join;
use graphql_client::{Response, GraphQLQuery};
use log::{debug, error, info};
use reqwest::Client;

// GraphQL DateTime scalar type.
type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/schema.graphql",
    query_path = "src/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug"
)]
pub struct UserActivity;

/// Generic helper function to fetch all nodes from a paginated connection.
/// - `build_vars`: a closure that accepts an optional cursor and returns query variables.
/// - `extract`: a closure that extracts (Option<Vec<T>>, &P) from ResponseData.
/// - `extract_page_info`: a closure that converts a reference to page info (of type P) into (Option<String>, bool).
async fn fetch_all_nodes<T, P>(
    client: &Client,
    build_vars: impl Fn(Option<String>) -> user_activity::Variables,
    extract: impl Fn(&user_activity::ResponseData) -> (&Option<Vec<T>>, &P),
    extract_page_info: impl Fn(&P) -> (Option<String>, bool),
) -> Result<Vec<T>>
where
    T: Clone,
{
    let mut all_nodes = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let variables = build_vars(cursor.clone());
        let request_body = UserActivity::build_query(variables);
        debug!("Pagination request: {:?}", request_body);

        let res = client
            .post("https://api.github.com/graphql")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send pagination request")?;
        info!("Pagination request sent, awaiting response.");

        let response_body: Response<user_activity::ResponseData> = res
            .json()
            .await
            .context("Failed to parse pagination response")?;
        debug!("Pagination response: {:?}", response_body);

        if let Some(errors) = response_body.errors {
            error!("GraphQL pagination errors: {:?}", errors);
            bail!("GraphQL pagination errors: {:?}", errors);
        }

        let data = response_body
            .data
            .ok_or_else(|| anyhow::anyhow!("No data received in pagination response"))?;
        let (nodes_opt, page_info) = extract(&data);
        if let Some(nodes) = nodes_opt {
            debug!("Fetched {} nodes", nodes.len());
            all_nodes.extend(nodes.clone());
        } else {
            debug!("No nodes found in this page");
        }
        let (end_cursor, has_next_page) = extract_page_info(page_info);
        if has_next_page {
            debug!("Has next page; setting cursor to {:?}", end_cursor);
            cursor = end_cursor;
        } else {
            info!("No further pages; pagination complete.");
            break;
        }
    }
    Ok(all_nodes)
}

/// Fetch all issue contribution nodes.
async fn fetch_issue_nodes(
    client: &Client,
    username: &str,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
    first: i64,
) -> Result<Vec<user_activity::UserActivityUserContributionsCollectionIssueContributionsNodes>> {
    fetch_all_nodes(
        client,
        |cursor| user_activity::Variables {
            username: username.to_string(),
            from: start_date.to_rfc3339(),
            to: end_date.to_rfc3339(),
            issues_first: first,
            issues_after: cursor,
            prs_first: first,           // Dummy values for unused fields.
            prs_after: None,
            pr_reviews_first: first,
            pr_reviews_after: None,
        },
        |data| {
            let issue_conn = &data.user.as_ref().unwrap().contributions_collection.issue_contributions;
            (&issue_conn.nodes, &issue_conn.page_info)
        },
        |page_info: &user_activity::UserActivityUserContributionsCollectionIssueContributionsPageInfo| {
            (page_info.end_cursor.clone(), page_info.has_next_page)
        },
    )
    .await
}

/// Fetch all pull request contribution nodes.
async fn fetch_pr_nodes(
    client: &Client,
    username: &str,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
    first: i64,
) -> Result<Vec<user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodes>> {
    fetch_all_nodes(
        client,
        |cursor| user_activity::Variables {
            username: username.to_string(),
            from: start_date.to_rfc3339(),
            to: end_date.to_rfc3339(),
            issues_first: first,
            issues_after: None,
            prs_first: first,
            prs_after: cursor,
            pr_reviews_first: first,
            pr_reviews_after: None,
        },
        |data| {
            let pr_conn = &data.user.as_ref().unwrap().contributions_collection.pull_request_contributions;
            (&pr_conn.nodes, &pr_conn.page_info)
        },
        |page_info: &user_activity::UserActivityUserContributionsCollectionPullRequestContributionsPageInfo| {
            (page_info.end_cursor.clone(), page_info.has_next_page)
        },
    )
    .await
}

/// Fetch all pull request review contribution nodes.
async fn fetch_pr_review_nodes(
    client: &Client,
    username: &str,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
    first: i64,
) -> Result<Vec<user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodes>> {
    fetch_all_nodes(
        client,
        |cursor| user_activity::Variables {
            username: username.to_string(),
            from: start_date.to_rfc3339(),
            to: end_date.to_rfc3339(),
            issues_first: first,
            issues_after: None,
            prs_first: first,
            prs_after: None,
            pr_reviews_first: first,
            pr_reviews_after: cursor,
        },
        |data| {
            let pr_review_conn = &data.user.as_ref().unwrap().contributions_collection.pull_request_review_contributions;
            (&pr_review_conn.nodes, &pr_review_conn.page_info)
        },
        |page_info: &user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsPageInfo| {
            (page_info.end_cursor.clone(), page_info.has_next_page)
        },
    )
    .await
}

/// Main fetch_activity function that fetches base data and concurrently fetches paginated nodes.
pub async fn fetch_activity(
    client: &Client,
    username: &str,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
) -> Result<user_activity::ResponseData> {
    let first = 10;

    // Fetch base data (non-paginated fields).
    let base_variables = user_activity::Variables {
        username: username.to_string(),
        from: start_date.to_rfc3339(),
        to: end_date.to_rfc3339(),
        issues_first: first,
        issues_after: None,
        prs_first: first,
        prs_after: None,
        pr_reviews_first: first,
        pr_reviews_after: None,
    };

    let base_request = UserActivity::build_query(base_variables);
    debug!("Base GraphQL request: {:?}", base_request);

    let res = client
        .post("https://api.github.com/graphql")
        .json(&base_request)
        .send()
        .await
        .context("Failed to send base request")?;
    let response_body: Response<user_activity::ResponseData> = res
        .json()
        .await
        .context("Failed to parse base response")?;
    if let Some(errors) = response_body.errors {
        bail!("GraphQL errors in base request: {:?}", errors);
    }
    let mut base_data = response_body
        .data
        .ok_or_else(|| anyhow::anyhow!("No data received in base response"))?;

    // Run paginated queries concurrently.
    let (issues, prs, pr_reviews) = join!(
        fetch_issue_nodes(client, username, start_date, end_date, first),
        fetch_pr_nodes(client, username, start_date, end_date, first),
        fetch_pr_review_nodes(client, username, start_date, end_date, first)
    );
    let issues = issues.context("Failed to fetch issue nodes")?;
    let prs = prs.context("Failed to fetch PR nodes")?;
    let pr_reviews = pr_reviews.context("Failed to fetch PR review nodes")?;

    // Replace the connection nodes in base_data with the accumulated results.
    if let Some(ref mut user) = base_data.user {
        user.contributions_collection.issue_contributions.nodes = Some(issues);
        user.contributions_collection.pull_request_contributions.nodes = Some(prs);
        user.contributions_collection.pull_request_review_contributions.nodes = Some(pr_reviews);
    }

    info!("All pagination complete; returning merged data.");
    Ok(base_data)
}