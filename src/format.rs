#![warn(missing_docs)]
//! Formatting module: defines a trait to format GitHub activity data into various output styles.

use crate::github::user_activity;
use chrono::{DateTime as ChronoDateTime, Utc};

/// A trait for formatting GitHub activity data.
pub trait FormatData {
    /// Formats the activity data given the time range and username.
    fn format(
        &self,
        activity: &user_activity::ResponseData,
        start_date: ChronoDateTime<Utc>,
        end_date: ChronoDateTime<Utc>,
        username: &str,
    ) -> String;
}

/// A plain text formatter for GitHub activity.
pub struct PlainTextFormatter;

impl FormatData for PlainTextFormatter {
    fn format(
        &self,
        activity: &user_activity::ResponseData,
        start_date: ChronoDateTime<Utc>,
        end_date: ChronoDateTime<Utc>,
        username: &str,
    ) -> String {
        let mut output = String::new();
        if let Some(user) = &activity.user {
            let cc = &user.contributions_collection;
            output.push_str(&format!("User: {}\n", username));
            output.push_str(&format!(
                "Time Period: {} to {}\n",
                start_date.to_rfc3339(),
                end_date.to_rfc3339()
            ));
            output.push_str(&format!(
                "Total Commit Contributions: {}\n",
                cc.total_commit_contributions
            ));
            output.push_str(&format!(
                "Total Issue Contributions: {}\n",
                cc.total_issue_contributions
            ));
            output.push_str(&format!(
                "Total Pull Request Contributions: {}\n",
                cc.total_pull_request_contributions
            ));
            output.push_str(&format!(
                "Total Pull Request Review Contributions: {}\n\n",
                cc.total_pull_request_review_contributions
            ));

            // Contribution Calendar
            output.push_str("Contribution Calendar:\n");
            output.push_str(&format!(
                "  Total Contributions: {}\n",
                cc.contribution_calendar.total_contributions
            ));
            for week in &cc.contribution_calendar.weeks {
                for day in &week.contribution_days {
                    output.push_str(&format!(
                        "    {}: {} contributions (weekday {})\n",
                        day.date, day.contribution_count, day.weekday
                    ));
                }
            }
            output.push('\n');

            // Repository Contributions
            output.push_str("Repository Contributions:\n");
            for repo_contrib in &cc.commit_contributions_by_repository {
                output.push_str(&format!(
                    "- {}: {} commits\n",
                    repo_contrib.repository.name_with_owner, repo_contrib.contributions.total_count
                ));
            }
            output.push('\n');

            // Issue Contributions
            output.push_str("Issue Contributions:\n");
            if let Some(nodes) = &cc.issue_contributions.nodes {
                for node in nodes {
                    let issue = &node.issue;
                    output.push_str(&format!(
                        "- Issue #{}: {}\n  URL: {}\n  Created: {}\n  State: {}\n  Closed: {:?}\n",
                        issue.number,
                        issue.title,
                        issue.url,
                        issue.created_at,
                        issue.state,
                        issue.closed_at
                    ));
                }
            }
            output.push('\n');

            // Pull Request Contributions
            output.push_str("Pull Request Contributions:\n");
            if let Some(nodes) = &cc.pull_request_contributions.nodes {
                for node in nodes {
                    let pr = &node.pull_request;
                    output.push_str(&format!(
                        "- PR #{}: {}\n  URL: {}\n  Created: {}\n  State: {}\n  Merged: {}\n  Merged At: {:?}\n  Closed: {:?}\n",
                        pr.number,
                        pr.title,
                        pr.url,
                        pr.created_at,
                        pr.state,
                        pr.merged,
                        pr.merged_at,
                        pr.closed_at
                    ));
                }
            }
            output.push('\n');

            // Pull Request Review Contributions
            output.push_str("Pull Request Review Contributions:\n");
            if let Some(nodes) = &cc.pull_request_review_contributions.nodes {
                for node in nodes {
                    let pr_review = &node.pull_request_review;
                    output.push_str(&format!(
                        "- PR Review for PR #{}: {}\n  URL: {}\n  Occurred At: {}\n",
                        pr_review.pull_request.number,
                        pr_review.pull_request.title,
                        pr_review.pull_request.url,
                        node.occurred_at
                    ));
                }
            }
        } else {
            output.push_str("No user data available.\n");
        }
        output
    }
}

/// A Markdown formatter for GitHub activity.
pub struct MarkdownFormatter;

impl FormatData for MarkdownFormatter {
    fn format(
        &self,
        activity: &user_activity::ResponseData,
        start_date: ChronoDateTime<Utc>,
        end_date: ChronoDateTime<Utc>,
        username: &str,
    ) -> String {
        let mut output = String::new();
        if let Some(user) = &activity.user {
            let cc = &user.contributions_collection;
            output.push_str(&format!("# GitHub Activity Report for {}\n\n", username));
            output.push_str(&format!(
                "**Time Period:** {} to {}\n\n",
                start_date.to_rfc3339(),
                end_date.to_rfc3339()
            ));
            output.push_str("## Summary\n\n");
            output.push_str(&format!(
                "- **Total Commit Contributions:** {}\n",
                cc.total_commit_contributions
            ));
            output.push_str(&format!(
                "- **Total Issue Contributions:** {}\n",
                cc.total_issue_contributions
            ));
            output.push_str(&format!(
                "- **Total Pull Request Contributions:** {}\n",
                cc.total_pull_request_contributions
            ));
            output.push_str(&format!(
                "- **Total Pull Request Review Contributions:** {}\n\n",
                cc.total_pull_request_review_contributions
            ));

            // Contribution Calendar
            output.push_str("## Contribution Calendar\n\n");
            output.push_str(&format!(
                "**Total Contributions:** {}\n\n",
                cc.contribution_calendar.total_contributions
            ));
            for week in &cc.contribution_calendar.weeks {
                for day in &week.contribution_days {
                    output.push_str(&format!(
                        "* {}: {} contributions (weekday {})\n",
                        day.date, day.contribution_count, day.weekday
                    ));
                }
            }
            output.push('\n');

            // Repository Contributions
            output.push_str("## Repository Contributions\n\n");
            output.push_str("| Repository             | Commits |\n");
            output.push_str("|------------------------|---------|\n");
            for repo_contrib in &cc.commit_contributions_by_repository {
                output.push_str(&format!(
                    "| {:<22} | {:>7} |\n",
                    repo_contrib.repository.name_with_owner, repo_contrib.contributions.total_count
                ));
            }
            output.push('\n');

            // Issue Contributions
            output.push_str("## Issue Contributions\n\n");
            output.push_str("| Issue # | Title | URL | Created At | State | Closed At |\n");
            output.push_str("|---------|-------|-----|------------|-------|-----------|\n");
            if let Some(nodes) = &cc.issue_contributions.nodes {
                for node in nodes {
                    let issue = &node.issue;
                    output.push_str(&format!(
                        "| {} | {} | {} | {} | {} | {} |\n",
                        issue.number,
                        issue.title,
                        issue.url,
                        issue.created_at,
                        issue.state,
                        issue.closed_at.as_deref().unwrap_or("N/A")
                    ));
                }
            }
            output.push('\n');

            // Pull Request Contributions
            output.push_str("## Pull Request Contributions\n\n");
            output.push_str(
                "| PR # | Title | URL | Created At | State | Merged | Merged At | Closed At |\n",
            );
            output.push_str(
                "|------|-------|-----|------------|-------|--------|-----------|-----------|\n",
            );
            if let Some(nodes) = &cc.pull_request_contributions.nodes {
                for node in nodes {
                    let pr = &node.pull_request;
                    output.push_str(&format!(
                        "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
                        pr.number,
                        pr.title,
                        pr.url,
                        pr.created_at,
                        pr.state,
                        pr.merged,
                        pr.merged_at.as_deref().unwrap_or("N/A"),
                        pr.closed_at.as_deref().unwrap_or("N/A")
                    ));
                }
            }
            output.push('\n');

            // Pull Request Review Contributions
            output.push_str("## Pull Request Review Contributions\n\n");
            output.push_str("| PR # | Title | URL | Occurred At |\n");
            output.push_str("|------|-------|-----|-------------|\n");
            if let Some(nodes) = &cc.pull_request_review_contributions.nodes {
                for node in nodes {
                    let pr_review = &node.pull_request_review;
                    output.push_str(&format!(
                        "| {} | {} | {} | {} |\n",
                        pr_review.pull_request.number,
                        pr_review.pull_request.title,
                        pr_review.pull_request.url,
                        node.occurred_at
                    ));
                }
            }
        } else {
            output.push_str("No user data available.\n");
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::user_activity;
    use chrono::{TimeZone, Utc};

    fn dummy_response_data() -> user_activity::ResponseData {
        user_activity::ResponseData {
            user: Some(user_activity::UserActivityUser {
                contributions_collection: user_activity::UserActivityUserContributionsCollection {
                    total_commit_contributions: 10,
                    total_issue_contributions: 5,
                    total_pull_request_contributions: 3,
                    total_pull_request_review_contributions: 2,
                    contribution_calendar: user_activity::UserActivityUserContributionsCollectionContributionCalendar {
                        total_contributions: 20,
                        weeks: vec![
                            user_activity::UserActivityUserContributionsCollectionContributionCalendarWeeks {
                                contribution_days: vec![
                                    user_activity::UserActivityUserContributionsCollectionContributionCalendarWeeksContributionDays {
                                        date: "2025-03-11T00:00:00Z".into(),
                                        contribution_count: 1,
                                        weekday: 2,
                                    },
                                ],
                            },
                        ],
                    },
                    commit_contributions_by_repository: vec![
                        user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepository {
                            repository: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryRepository {
                                name_with_owner: "owner/repo".into(),
                                updated_at: "2025-03-10T00:00:00Z".into(),
                            },
                            contributions: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryContributions {
                                total_count: 5,
                            },
                        },
                    ],
                    issue_contributions: user_activity::UserActivityUserContributionsCollectionIssueContributions {
                        total_count: 1,
                        page_info: user_activity::UserActivityUserContributionsCollectionIssueContributionsPageInfo {
                            end_cursor: None,
                            has_next_page: false,
                        },
                        nodes: Some(vec![
                            user_activity::UserActivityUserContributionsCollectionIssueContributionsNodes {
                                issue: user_activity::UserActivityUserContributionsCollectionIssueContributionsNodesIssue {
                                    number: 42,
                                    title: "Test Issue".into(),
                                    url: "http://example.com/issue".into(),
                                    created_at: "2025-03-09T00:00:00Z".into(),
                                    state: "open".into(),
                                    closed_at: None,
                                },
                            },
                        ]),
                    },
                    pull_request_contributions: user_activity::UserActivityUserContributionsCollectionPullRequestContributions {
                        total_count: 1,
                        page_info: user_activity::UserActivityUserContributionsCollectionPullRequestContributionsPageInfo {
                            end_cursor: None,
                            has_next_page: false,
                        },
                        nodes: Some(vec![
                            user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodes {
                                pull_request: user_activity::UserActivityUserContributionsCollectionPullRequestContributionsNodesPullRequest {
                                    number: 101,
                                    title: "Test PR".into(),
                                    url: "http://example.com/pr".into(),
                                    created_at: "2025-03-08T00:00:00Z".into(),
                                    state: "closed".into(),
                                    merged: false,
                                    merged_at: None,
                                    closed_at: None,
                                },
                            },
                        ]),
                    },
                    pull_request_review_contributions: user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributions {
                        total_count: 1,
                        page_info: user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsPageInfo {
                            end_cursor: None,
                            has_next_page: false,
                        },
                        nodes: Some(vec![
                            user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodes {
                                pull_request_review: user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodesPullRequestReview {
                                    pull_request: user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsNodesPullRequestReviewPullRequest {
                                        number: 202,
                                        title: "Test PR Review".into(),
                                        url: "http://example.com/pr_review".into(),
                                    },
                                },
                                occurred_at: "2025-03-07T00:00:00Z".into(),
                            },
                        ]),
                    },
                },
            }),
        }
    }

    #[test]
    fn test_format_plain_contains_required_data() {
        let start_date = Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();
        let end_date = Utc.with_ymd_and_hms(2025, 3, 12, 0, 0, 0).unwrap();
        let data = dummy_response_data();
        let output = PlainTextFormatter.format(&data, start_date, end_date, "dummy");

        // Check for header and time period.
        assert!(output.contains("User: dummy"));
        assert!(output.contains("Time Period:"));
        assert!(output.contains(&format!(
            "{} to {}",
            start_date.to_rfc3339(),
            end_date.to_rfc3339()
        )));

        // Check summary details.
        assert!(output.contains("Total Commit Contributions: 10"));
        assert!(output.contains("Total Issue Contributions: 5"));
        assert!(output.contains("Total Pull Request Contributions: 3"));
        assert!(output.contains("Total Pull Request Review Contributions: 2"));

        // Check contribution calendar.
        assert!(output.contains("Contribution Calendar:"));
        assert!(output.contains("Total Contributions: 20"));
        assert!(output.contains("2025-03-11T00:00:00Z: 1 contributions (weekday 2)"));

        // Check repository contributions.
        assert!(output.contains("Repository Contributions:"));
        assert!(output.contains("owner/repo"));
        assert!(output.contains("5 commits"));

        // Check issue contributions.
        assert!(output.contains("Issue Contributions:"));
        assert!(output.contains("Issue #42: Test Issue"));
        assert!(output.contains("http://example.com/issue"));

        // Check pull request contributions.
        assert!(output.contains("Pull Request Contributions:"));
        assert!(output.contains("PR #101: Test PR"));
        assert!(output.contains("http://example.com/pr"));

        // Check pull request review contributions.
        assert!(output.contains("Pull Request Review Contributions:"));
        assert!(output.contains("PR Review for PR #202: Test PR Review"));
        assert!(output.contains("http://example.com/pr_review"));
    }

    #[test]
    fn test_format_markdown_contains_required_data() {
        let start_date = Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();
        let end_date = Utc.with_ymd_and_hms(2025, 3, 12, 0, 0, 0).unwrap();
        let data = dummy_response_data();
        let output = MarkdownFormatter.format(&data, start_date, end_date, "dummy");

        // Check header and time period.
        assert!(output.contains("# GitHub Activity Report for dummy"));
        assert!(output.contains("**Time Period:**"));
        assert!(output.contains(&format!(
            "{} to {}",
            start_date.to_rfc3339(),
            end_date.to_rfc3339()
        )));

        // Check summary details.
        assert!(output.contains("- **Total Commit Contributions:** 10"));
        assert!(output.contains("- **Total Issue Contributions:** 5"));
        assert!(output.contains("- **Total Pull Request Contributions:** 3"));
        assert!(output.contains("- **Total Pull Request Review Contributions:** 2"));

        // Check contribution calendar.
        assert!(output.contains("## Contribution Calendar"));
        assert!(output.contains("**Total Contributions:** 20"));
        assert!(output.contains("* 2025-03-11T00:00:00Z: 1 contributions (weekday 2)"));

        // Check repository contributions table.
        assert!(output.contains("## Repository Contributions"));
        assert!(output.contains("| Repository"));
        assert!(output.contains("owner/repo"));
        assert!(output.contains("5"));

        // Check issue contributions table.
        assert!(output.contains("## Issue Contributions"));
        assert!(output.contains("| Issue #"));
        assert!(output.contains("Test Issue"));
        assert!(output.contains("http://example.com/issue"));

        // Check pull request contributions table.
        assert!(output.contains("## Pull Request Contributions"));
        assert!(output.contains("| PR #"));
        assert!(output.contains("Test PR"));
        assert!(output.contains("http://example.com/pr"));

        // Check pull request review contributions table.
        assert!(output.contains("## Pull Request Review Contributions"));
        assert!(output.contains("Test PR Review"));
        assert!(output.contains("http://example.com/pr_review"));
    }
}
