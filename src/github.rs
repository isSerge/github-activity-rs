use anyhow::{Context, Result, bail};
use chrono::{DateTime as ChronoDateTime, Utc};
use futures::join;
use graphql_client::{GraphQLQuery, Response};
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
            .post(
                std::env::var("GITHUB_GRAPHQL_URL")
                    .unwrap_or_else(|_| "https://api.github.com/graphql".into()),
            )
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
) -> Result<Vec<user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodes>>
{
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
) -> Result<
    Vec<user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodes>,
> {
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

    let graphql_url = std::env::var("GITHUB_GRAPHQL_URL")
        .unwrap_or_else(|_| "https://api.github.com/graphql".into());

    let res = client
        .post(&graphql_url)
        .json(&base_request)
        .send()
        .await
        .context("Failed to send base request")?;

    let response_body: Response<user_activity::ResponseData> =
        res.json().await.context("Failed to parse base response")?;
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
        user.contributions_collection
            .pull_request_contributions
            .nodes = Some(prs);
        user.contributions_collection
            .pull_request_review_contributions
            .nodes = Some(pr_reviews);
    }

    info!("All pagination complete; returning merged data.");
    Ok(base_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use reqwest::Client;
    use serde_json::json;
    use serial_test::serial;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    // Helper: Build a full response containing all three connections.
    // For the connection of interest, we provide Some(node) and specific pageInfo.
    // For the others, we supply dummy empty responses.
    fn build_full_response(
        issue: Option<serde_json::Value>,
        issue_page_info: serde_json::Value,
        pr: Option<serde_json::Value>,
        pr_page_info: serde_json::Value,
        pr_review: Option<serde_json::Value>,
        pr_review_page_info: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "data": {
                "user": {
                    "contributionsCollection": {
                        "totalCommitContributions": 0,
                        "totalIssueContributions": 0,
                        "totalPullRequestContributions": 0,
                        "totalPullRequestReviewContributions": 0,
                        "contributionCalendar": {
                            "totalContributions": 0,
                            "weeks": []
                        },
                        "commitContributionsByRepository": [],
                        "issueContributions": {
                            "totalCount": if issue.is_some() { 2 } else { 0 },
                            "pageInfo": issue_page_info,
                            "nodes": if let Some(v) = issue { vec![v] } else { vec![] }
                        },
                        "pullRequestContributions": {
                            "totalCount": if pr.is_some() { 2 } else { 0 },
                            "pageInfo": pr_page_info,
                            "nodes": if let Some(v) = pr { vec![v] } else { vec![] }
                        },
                        "pullRequestReviewContributions": {
                            "totalCount": if pr_review.is_some() { 2 } else { 0 },
                            "pageInfo": pr_review_page_info,
                            "nodes": if let Some(v) = pr_review { vec![v] } else { vec![] }
                        }
                    }
                }
            }
        })
    }

    #[tokio::test]
    #[serial]
    async fn test_fetch_issue_nodes_pagination() {
        // Start a mock server.
        let mock_server = wiremock::MockServer::start().await;

        // Build two fake responses for pagination.
        let response_page1 = build_full_response(
            Some(json!({
                "issue": {
                    "number": 1,
                    "title": "Issue 1",
                    "url": "http://example.com/issue1",
                    "createdAt": "2025-03-01T00:00:00Z",
                    "state": "open",
                    "closedAt": null,
                    "repository": {
                        "nameWithOwner": "owner/repo1",
                        "updatedAt": "2025-03-01T00:00:00Z"
                    }
                }
            })),
            json!({
                "endCursor": "cursor1",
                "hasNextPage": true
            }),
            None, // dummy for PR
            json!({ "endCursor": null, "hasNextPage": false }),
            None, // dummy for PR reviews
            json!({ "endCursor": null, "hasNextPage": false }),
        );
        let response_page2 = build_full_response(
            Some(json!({
                "issue": {
                    "number": 2,
                    "title": "Issue 2",
                    "url": "http://example.com/issue2",
                    "createdAt": "2025-03-02T00:00:00Z",
                    "state": "closed",
                    "closedAt": "2025-03-03T00:00:00Z",
                    "repository": {
                        "nameWithOwner": "owner/repo2",
                        "updatedAt": "2025-03-02T00:00:00Z"
                    }
                }
            })),
            json!({
                "endCursor": null,
                "hasNextPage": false
            }),
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        // Use an atomic counter to keep track of the number of calls.
        let call_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = call_counter.clone();

        // Mount a single mock that returns different responses based on the call count.
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(move |_request: &wiremock::Request| {
                let call_num = counter_clone.fetch_add(1, Ordering::SeqCst);
                if call_num == 0 {
                    ResponseTemplate::new(200).set_body_json(response_page1.clone())
                } else if call_num == 1 {
                    ResponseTemplate::new(200).set_body_json(response_page2.clone())
                } else {
                    // Fallback: return a valid (but empty) response.
                    ResponseTemplate::new(200).set_body_string("{\"data\":{\"user\":null}}")
                }
            })
            .mount(&mock_server)
            .await;

        // Override the URL by setting the environment variable.
        unsafe {
            std::env::set_var(
                "GITHUB_GRAPHQL_URL",
                format!("{}/graphql", mock_server.uri()),
            );
        }

        let client = Client::new();

        // Define a dummy build_vars closure.
        let build_vars = |cursor: Option<String>| user_activity::Variables {
            username: "dummy".into(),
            from: Utc::now().to_rfc3339(),
            to: Utc::now().to_rfc3339(),
            issues_first: 10,
            issues_after: cursor,
            prs_first: 10, // Dummy values; not used in this test.
            prs_after: None,
            pr_reviews_first: 10,
            pr_reviews_after: None,
        };

        // Define a function to extract issue contributions with explicit lifetimes.
        fn extract_issue<'a>(
            data: &'a user_activity::ResponseData,
        ) -> (
            &'a Option<
                Vec<user_activity::UserActivityUserContributionsCollectionIssueContributionsNodes>,
            >,
            &'a user_activity::UserActivityUserContributionsCollectionIssueContributionsPageInfo,
        ) {
            let issue_conn = &data
                .user
                .as_ref()
                .unwrap()
                .contributions_collection
                .issue_contributions;
            (&issue_conn.nodes, &issue_conn.page_info)
        }

        // Closure to extract the pagination info.
        let extract_page_info = |page_info: &user_activity::UserActivityUserContributionsCollectionIssueContributionsPageInfo| {
        (page_info.end_cursor.clone(), page_info.has_next_page)
    };

        // Call the fetch_all_nodes helper.
        let nodes = fetch_all_nodes::<
            user_activity::UserActivityUserContributionsCollectionIssueContributionsNodes,
            user_activity::UserActivityUserContributionsCollectionIssueContributionsPageInfo,
        >(&client, build_vars, extract_issue, extract_page_info)
        .await
        .unwrap();

        // Assert that we aggregated 2 nodes.
        assert_eq!(nodes.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_fetch_pr_nodes_pagination() {
        // This test focuses on pullRequestContributions.
        let mock_server = wiremock::MockServer::start().await;
        // Build two responses: first page with a next cursor, second page final.
        let response_page1 = build_full_response(
            None, // issueContributions: empty
            json!({ "endCursor": null, "hasNextPage": false }),
            Some(json!({
                "pullRequest": {
                    "number": 101,
                    "title": "PR 1",
                    "url": "http://example.com/pr1",
                    "createdAt": "2025-03-01T00:00:00Z",
                    "state": "open",
                    "merged": false,
                    "mergedAt": null,
                    "closedAt": null,
                    "repository": {
                        "nameWithOwner": "owner/repo1",
                        "updatedAt": "2025-03-01T00:00:00Z"
                    }
                }
            })),
            json!({ "endCursor": "pr_cursor1", "hasNextPage": true }),
            None, // pullRequestReviewContributions dummy empty
            json!({ "endCursor": null, "hasNextPage": false }),
        );
        let response_page2 = build_full_response(
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
            Some(json!({
                "pullRequest": {
                    "number": 102,
                    "title": "PR 2",
                    "url": "http://example.com/pr2",
                    "createdAt": "2025-03-02T00:00:00Z",
                    "state": "closed",
                    "merged": true,
                    "mergedAt": "2025-03-03T00:00:00Z",
                    "closedAt": "2025-03-04T00:00:00Z",
                    "repository": {
                        "nameWithOwner": "owner/repo2",
                        "updatedAt": "2025-03-02T00:00:00Z"
                    }
                }
            })),
            json!({ "endCursor": null, "hasNextPage": false }),
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        // Use an atomic counter to alternate responses.
        let call_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = call_counter.clone();
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(move |_req: &wiremock::Request| {
                let call_num = counter_clone.fetch_add(1, Ordering::SeqCst);
                if call_num == 0 {
                    ResponseTemplate::new(200).set_body_json(response_page1.clone())
                } else if call_num == 1 {
                    ResponseTemplate::new(200).set_body_json(response_page2.clone())
                } else {
                    ResponseTemplate::new(200).set_body_string("{\"data\":{\"user\":null}}")
                }
            })
            .mount(&mock_server)
            .await;

        unsafe {
            std::env::set_var(
                "GITHUB_GRAPHQL_URL",
                format!("{}/graphql", mock_server.uri()),
            );
        }

        let client = Client::new();
        let build_vars = |cursor: Option<String>| user_activity::Variables {
            username: "dummy".into(),
            from: Utc::now().to_rfc3339(),
            to: Utc::now().to_rfc3339(),
            issues_first: 10,
            issues_after: cursor.clone(),
            prs_first: 10,
            prs_after: cursor.clone(), // Notice: for PR nodes, we pass the cursor.
            pr_reviews_first: 10,
            pr_reviews_after: None,
        };

        fn extract_pr<'a>(
            data: &'a user_activity::ResponseData,
        ) -> (
            &'a Option<Vec<user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodes>>,
            &'a user_activity::UserActivityUserContributionsCollectionPullRequestContributionsPageInfo,
        ){
            let pr_conn = &data
                .user
                .as_ref()
                .unwrap()
                .contributions_collection
                .pull_request_contributions;
            (&pr_conn.nodes, &pr_conn.page_info)
        }

        let extract_page_info = |page_info: &user_activity::UserActivityUserContributionsCollectionPullRequestContributionsPageInfo| {
            (page_info.end_cursor.clone(), page_info.has_next_page)
        };

        let nodes = fetch_all_nodes::<
            user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodes,
            user_activity::UserActivityUserContributionsCollectionPullRequestContributionsPageInfo,
        >(&client, build_vars, extract_pr, extract_page_info)
        .await
        .unwrap();

        debug!("Fetched PR nodes: {:?}", nodes);
        assert_eq!(
            nodes.len(),
            2,
            "Expected 2 PR nodes but got {}",
            nodes.len()
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_fetch_pr_review_nodes_pagination() {
        // This test focuses on pullRequestReviewContributions.
        let mock_server = wiremock::MockServer::start().await;

        // Build two responses: first page with a next cursor, second page final.
        let response_page1 = build_full_response(
            None, // issueContributions: empty
            json!({ "endCursor": null, "hasNextPage": false }),
            None, // pullRequestContributions: empty
            json!({ "endCursor": null, "hasNextPage": false }),
            Some(json!({
                "occurredAt": "2025-03-01T00:00:00Z",
                "pullRequestReview": {
                    "createdAt": "2025-03-01T00:00:00Z",
                    "pullRequest": {
                        "number": 101,
                        "title": "PR 1",
                        "url": "http://example.com/pr1",
                        "createdAt": "2025-03-01T00:00:00Z",
                        "state": "open"
                    }
                }
            })),
            json!({ "endCursor": "pr_review_cursor1", "hasNextPage": true }),
        );
        let response_page2 = build_full_response(
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
            Some(json!({
                "occurredAt": "2025-03-02T00:00:00Z",
                "pullRequestReview": {
                    "createdAt": "2025-03-02T00:00:00Z",
                    "pullRequest": {
                        "number": 102,
                        "title": "PR 2",
                        "url": "http://example.com/pr2",
                        "createdAt": "2025-03-02T00:00:00Z",
                        "state": "closed"
                    }
                }
            })),
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        // Use an atomic counter to alternate responses.
        let call_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = call_counter.clone();
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(move |_req: &wiremock::Request| {
                let call_num = counter_clone.fetch_add(1, Ordering::SeqCst);
                if call_num == 0 {
                    ResponseTemplate::new(200).set_body_json(response_page1.clone())
                } else if call_num == 1 {
                    ResponseTemplate::new(200).set_body_json(response_page2.clone())
                } else {
                    ResponseTemplate::new(200).set_body_string("{\"data\":{\"user\":null}}")
                }
            })
            .mount(&mock_server)
            .await;

        unsafe {
            std::env::set_var(
                "GITHUB_GRAPHQL_URL",
                format!("{}/graphql", mock_server.uri()),
            );
        }

        let client = Client::new();
        let build_vars = |cursor: Option<String>| user_activity::Variables {
            username: "dummy".into(),
            from: Utc::now().to_rfc3339(),
            to: Utc::now().to_rfc3339(),
            issues_first: 10,
            issues_after: cursor.clone(),
            prs_first: 10,
            prs_after: None,
            pr_reviews_first: 10,
            pr_reviews_after: cursor.clone(),
        };

        fn extract_pr_review<'a>(
            data: &'a user_activity::ResponseData,
        ) -> (
            &'a Option<Vec<user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodes>>,
            &'a user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsPageInfo,
        ){
            let pr_review_conn = &data
                .user
                .as_ref()
                .unwrap()
                .contributions_collection
                .pull_request_review_contributions;
            (&pr_review_conn.nodes, &pr_review_conn.page_info)
        }

        let extract_page_info = |page_info: &user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsPageInfo| {
            (page_info.end_cursor.clone(), page_info.has_next_page)
        };

        let nodes = fetch_all_nodes::<
            user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodes,
            user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsPageInfo,
        >(&client, build_vars, extract_pr_review, extract_page_info)
        .await
        .unwrap();

        debug!("Fetched PR review nodes: {:?}", nodes);

        assert_eq!(
            nodes.len(),
            2,
            "Expected 2 PR review nodes but got {}",
            nodes.len()
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_fetch_activity_base_error() {
        // Start a mock server (isolated for this test).
        let mock_server = wiremock::MockServer::start().await;

        // Build a fake error response for the base query.
        let error_response = serde_json::json!({
            "data": null,
            "errors": [
                { "message": "Base request error" }
            ]
        });

        // Mount a mock to return the error response.
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(error_response))
            .mount(&mock_server)
            .await;

        unsafe {
            std::env::set_var(
                "GITHUB_GRAPHQL_URL",
                format!("{}/graphql", mock_server.uri()),
            );
        }

        let client = Client::new();
        let start_date = Utc::now();
        let end_date = Utc::now();

        // Call fetch_activity, which should fail because the base response contains errors.
        let result = fetch_activity(&client, "dummy", start_date, end_date).await;

        assert!(
            result.is_err(),
            "Expected fetch_activity to fail due to base query errors"
        );

        let err_str = format!("{:?}", result.err().unwrap());
        assert!(
            err_str.contains("GraphQL errors in base request"),
            "Error message did not contain expected text: {}",
            err_str
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_fetch_activity_merge_data() {
        // Start a mock server (isolated for this test).
        let mock_server = wiremock::MockServer::start().await;

        // Build a base response that contains valid non-paginated fields (empty node arrays).
        let base_response = json!({
            "data": {
                "user": {
                    "contributionsCollection": {
                        "totalCommitContributions": 5,
                        "totalIssueContributions": 0,
                        "totalPullRequestContributions": 0,
                        "totalPullRequestReviewContributions": 0,
                        "contributionCalendar": {
                            "totalContributions": 5,
                            "weeks": []
                        },
                        "commitContributionsByRepository": [],
                        "issueContributions": {
                            "totalCount": 0,
                            "pageInfo": { "endCursor": null, "hasNextPage": false },
                            "nodes": []
                        },
                        "pullRequestContributions": {
                            "totalCount": 0,
                            "pageInfo": { "endCursor": null, "hasNextPage": false },
                            "nodes": []
                        },
                        "pullRequestReviewContributions": {
                            "totalCount": 0,
                            "pageInfo": { "endCursor": null, "hasNextPage": false },
                            "nodes": []
                        }
                    }
                }
            }
        });

        // Use the helper already defined in your tests to build a full response.
        // For paginated responses, we simulate one page containing a single node each.
        let issue_response = build_full_response(
            Some(json!({
                "issue": {
                    "number": 1,
                    "title": "Issue 1",
                    "url": "http://example.com/issue1",
                    "createdAt": "2025-03-01T00:00:00Z",
                    "state": "open",
                    "closedAt": null,
                    "repository": {
                        "nameWithOwner": "owner/repo1",
                        "updatedAt": "2025-03-01T00:00:00Z"
                    }
                }
            })),
            json!({ "endCursor": null, "hasNextPage": false }),
            None, // pull request contributions dummy
            json!({ "endCursor": null, "hasNextPage": false }),
            None, // pull request review contributions dummy
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        let pr_response = build_full_response(
            None, // issue contributions dummy
            json!({ "endCursor": null, "hasNextPage": false }),
            Some(json!({
                "pullRequest": {
                    "number": 101,
                    "title": "PR 1",
                    "url": "http://example.com/pr1",
                    "createdAt": "2025-03-01T00:00:00Z",
                    "state": "open",
                    "merged": false,
                    "mergedAt": null,
                    "closedAt": null,
                    "repository": {
                        "nameWithOwner": "owner/repo1",
                        "updatedAt": "2025-03-01T00:00:00Z"
                    }
                }
            })),
            json!({ "endCursor": null, "hasNextPage": false }),
            None, // pull request review contributions dummy
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        let pr_review_response = build_full_response(
            None, // issue contributions dummy
            json!({ "endCursor": null, "hasNextPage": false }),
            None, // pull request contributions dummy
            json!({ "endCursor": null, "hasNextPage": false }),
            Some(json!({
                "occurredAt": "2025-03-01T00:00:00Z",
                "pullRequestReview": {
                    "createdAt": "2025-03-01T00:00:00Z",
                    "pullRequest": {
                        "number": 201,
                        "title": "Review 1",
                        "url": "http://example.com/prreview1",
                        "createdAt": "2025-03-01T00:00:00Z",
                        "state": "open"
                    }
                }
            })),
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        // Use an atomic counter so that we return responses in sequence.
        // We expect 4 POST requests in total: one base query and one for each pagination.
        let call_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = call_counter.clone();

        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(move |_req: &wiremock::Request| {
                let call_num = counter_clone.fetch_add(1, Ordering::SeqCst);
                match call_num {
                    0 => ResponseTemplate::new(200).set_body_json(base_response.clone()),
                    1 => ResponseTemplate::new(200).set_body_json(issue_response.clone()),
                    2 => ResponseTemplate::new(200).set_body_json(pr_response.clone()),
                    3 => ResponseTemplate::new(200).set_body_json(pr_review_response.clone()),
                    _ => ResponseTemplate::new(200).set_body_string("{\"data\":{\"user\":null}}"),
                }
            })
            .mount(&mock_server)
            .await;

        unsafe {
            std::env::set_var(
                "GITHUB_GRAPHQL_URL",
                format!("{}/graphql", mock_server.uri()),
            );
        }

        let client = Client::new();
        let start_date = Utc::now();
        let end_date = Utc::now();

        // Call fetch_activity which first gets the base response then runs the paginated queries concurrently.
        let merged_data = fetch_activity(&client, "dummy", start_date, end_date)
            .await
            .expect("fetch_activity failed");

        // Verify that the base data has been updated with the nodes from the paginated calls.
        let user = merged_data.user.expect("Expected user data");
        let contributions = user.contributions_collection;

        let issue_nodes = contributions
            .issue_contributions
            .nodes
            .expect("Expected issue nodes");
        let pr_nodes = contributions
            .pull_request_contributions
            .nodes
            .expect("Expected PR nodes");
        let pr_review_nodes = contributions
            .pull_request_review_contributions
            .nodes
            .expect("Expected PR review nodes");

        // We expect one node in each connection (replacing the base empty arrays).
        assert_eq!(issue_nodes.len(), 1, "Expected 1 issue node");
        assert_eq!(pr_nodes.len(), 1, "Expected 1 PR node");
        assert_eq!(pr_review_nodes.len(), 1, "Expected 1 PR review node");
    }
}
