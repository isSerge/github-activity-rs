#[cfg(test)]
mod tests {
    use crate::github::GithubClient;
    use chrono::Utc;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use temp_env::with_var;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate, MockServer};
    use tokio::runtime::Runtime;

    // Helper: Build a full response containing all three connections.
    // For the connection of interest, we provide Some(node) and specific pageInfo.
    // For the others, we supply dummy empty responses.
    fn build_full_response(
        issue: Option<Value>,
        issue_page_info: Value,
        pr: Option<Value>,
        pr_page_info: Value,
        pr_review: Option<Value>,
        pr_review_page_info: Value,
    ) -> Value {
        json!({
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

    // Helper to create a dummy GithubClient for testing.
    // We use a dummy token since wiremock intercepts the HTTP requests.
    fn create_test_client() -> GithubClient {
        let dummy_token = "dummy_token".to_string();
        let username = "dummy".to_string();
        let start_date = Utc::now();
        let end_date = Utc::now();
        GithubClient::new(dummy_token, username, start_date, end_date).unwrap()
    }

    #[test]
fn test_fetch_activity_base_error() {
    // Create an initial runtime for async setup.
    let rt = Runtime::new().unwrap();

    // Start the mock server and mount the error response.
    let mock_server = rt.block_on(async {
        let server = MockServer::start().await;
        let error_response = json!({
            "data": null,
            "errors": [
                { "message": "Base request error" }
            ]
        });
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(error_response))
            .mount(&server)
            .await;
        server
    });

    // Now that the server is set up, use temp_env::with_var (closure-based).
    with_var(
        "GITHUB_GRAPHQL_URL",
        Some(format!("{}/graphql", mock_server.uri())),
        || {
            // Create a fresh runtime inside the closure.
            let rt2 = Runtime::new().unwrap();
            rt2.block_on(async {
                let client = create_test_client();
                let result = client.fetch_activity().await;
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
            });
        },
    );
}
#[test]
fn test_fetch_activity_merge_data() {
    // Create an initial runtime for async setup.
    let rt = Runtime::new().unwrap();

    // Start the mock server and mount the responses.
    let mock_server = rt.block_on(async {
        let server = MockServer::start().await;

        // Build the base response.
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

        // Build paginated responses.
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
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        let pr_response = build_full_response(
            None,
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
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
        );

        let pr_review_response = build_full_response(
            None,
            json!({ "endCursor": null, "hasNextPage": false }),
            None,
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

        // Use an atomic counter to sequence the responses.
        let call_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = call_counter.clone();
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(move |_req: &wiremock::Request| {
                let call_num =
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                match call_num {
                    0 => ResponseTemplate::new(200).set_body_json(base_response.clone()),
                    1 => ResponseTemplate::new(200).set_body_json(issue_response.clone()),
                    2 => ResponseTemplate::new(200).set_body_json(pr_response.clone()),
                    3 => ResponseTemplate::new(200).set_body_json(pr_review_response.clone()),
                    _ => ResponseTemplate::new(200)
                        .set_body_string("{\"data\":{\"user\":null}}"),
                }
            })
            .mount(&server)
            .await;
        server
    });

    // Now use temp_env::with_var in a synchronous closure.
    with_var(
        "GITHUB_GRAPHQL_URL",
        Some(format!("{}/graphql", mock_server.uri())),
        || {
            // Create a new runtime inside the closure.
            let rt2 = Runtime::new().unwrap();
            rt2.block_on(async {
                let client = create_test_client();
                let merged_data = client
                    .fetch_activity()
                    .await
                    .expect("fetch_activity failed");
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

                assert_eq!(issue_nodes.len(), 1, "Expected 1 issue node");
                assert_eq!(pr_nodes.len(), 1, "Expected 1 PR node");
                assert_eq!(pr_review_nodes.len(), 1, "Expected 1 PR review node");
            });
        },
    );
  }
}