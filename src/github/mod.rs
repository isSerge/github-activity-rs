#[cfg(test)]
mod tests;

use anyhow::{Context, Result, bail};
use chrono::{DateTime as ChronoDateTime, Utc};
use futures::join;
use graphql_client::{GraphQLQuery, Response};
use log::{debug, error, info};
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};

// GraphQL DateTime scalar type.
type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/github.graphql",
    response_derives = "Debug, Default, serde::Serialize, Clone",
    variables_derives = "Debug"
)]
pub struct UserActivity;

pub struct GithubClient {
    client: Client,
    username: String,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
}

impl GithubClient {
    pub fn new(
        github_token: String,
        username: String,
        start_date: ChronoDateTime<Utc>,
        end_date: ChronoDateTime<Utc>,
    ) -> Result<Self> {
        // Build the HTTP client with the GitHub token.
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", github_token))
                .context("Failed to build authorization header")?,
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("github-activity-rs"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;
        debug!("HTTP client built successfully.");

        Ok(Self {
            client,
            username,
            start_date,
            end_date,
        })
    }

    /// Main fetch_activity function that fetches base data and concurrently fetches paginated nodes.
    pub async fn fetch_activity(&self) -> Result<user_activity::ResponseData> {
        let first = 10;

        // Fetch base data (non-paginated fields).
        let base_variables = user_activity::Variables {
            username: self.username.to_string(),
            from: self.start_date.to_rfc3339(),
            to: self.end_date.to_rfc3339(),
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

        let res = self
            .client
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
            self.fetch_issue_nodes(first),
            self.fetch_pr_nodes(first),
            self.fetch_pr_review_nodes(first)
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

    /// Generic helper function to fetch all nodes from a paginated connection.
    /// - `build_vars`: a closure that accepts an optional cursor and returns query variables.
    /// - `extract`: a closure that extracts (Option<Vec<T>>, &P) from ResponseData.
    /// - `extract_page_info`: a closure that converts a reference to page info (of type P) into (Option<String>, bool).
    async fn fetch_all_nodes<T, P>(
        &self,
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

            let res = self
                .client
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
        &self,
        first: i64,
    ) -> Result<Vec<user_activity::UserActivityUserContributionsCollectionIssueContributionsNodes>>
    {
        self.fetch_all_nodes(
          |cursor| user_activity::Variables {
              username: self.username.to_string(),
              from: self.start_date.to_rfc3339(),
              to: self.end_date.to_rfc3339(),
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
        &self,
        first: i64,
    ) -> Result<
        Vec<user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodes>,
    > {
        self.fetch_all_nodes(
          |cursor| user_activity::Variables {
              username: self.username.to_string(),
              from: self.start_date.to_rfc3339(),
              to: self.end_date.to_rfc3339(),
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
        &self,
        first: i64,
    ) -> Result<
        Vec<user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodes>,
    >{
        self.fetch_all_nodes(
          |cursor| user_activity::Variables {
              username: self.username.to_string(),
              from: self.start_date.to_rfc3339(),
              to: self.end_date.to_rfc3339(),
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
}
