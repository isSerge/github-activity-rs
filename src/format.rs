use chrono::{DateTime as ChronoDateTime, Utc};
use crate::github::user_activity;

/// Build a plain text report from the activity data.
pub fn format_plain(
    activity: &user_activity::ResponseData,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
    username: &str,
) -> String {
    let mut output = String::new();
    if let Some(user) = &activity.user {
        let cc = &user.contributions_collection;
        output.push_str(&format!("User: {}\n", username));
        output.push_str(&format!("Time Period: {} to {}\n", start_date.to_rfc3339(), end_date.to_rfc3339()));
        output.push_str(&format!("Total Commit Contributions: {}\n", cc.total_commit_contributions));
        output.push_str(&format!("Total Issue Contributions: {}\n", cc.total_issue_contributions));
        output.push_str(&format!("Total Pull Request Contributions: {}\n", cc.total_pull_request_contributions));
        output.push_str(&format!("Total Pull Request Review Contributions: {}\n\n", cc.total_pull_request_review_contributions));

        output.push_str("Repository Contributions:\n");
        for repo_contrib in &cc.commit_contributions_by_repository {
            output.push_str(&format!(
                "- {}: {}\n",
                repo_contrib.repository.name_with_owner,
                repo_contrib.contributions.total_count
            ));
        }
        output.push_str("\nIssue Contributions:\n");
        if let Some(issue_conn) = &user.contributions_collection.issue_contributions.nodes {
            for issue_node in issue_conn {
                output.push_str(&format!(
                    "- Issue #{}: {} (URL: {})\n",
                    issue_node.issue.number,
                    issue_node.issue.title,
                    issue_node.issue.url
                ));
            }
        }
        output.push_str("\nPull Request Contributions:\n");
        if let Some(pr_conn) = &user.contributions_collection.pull_request_contributions.nodes {
            for pr_node in pr_conn {
                output.push_str(&format!(
                    "- PR #{}: {} (URL: {})\n",
                    pr_node.pull_request.number,
                    pr_node.pull_request.title,
                    pr_node.pull_request.url
                ));
            }
        }
        output.push_str("\nPull Request Review Contributions:\n");
        if let Some(pr_review_conn) = &user.contributions_collection.pull_request_review_contributions.nodes {
            for pr_review in pr_review_conn {
                output.push_str(&format!(
                    "- PR Review for PR #{}: {} (URL: {}) - occurred at {}\n",
                    pr_review.pull_request_review.pull_request.number,
                    pr_review.pull_request_review.pull_request.title,
                    pr_review.pull_request_review.pull_request.url,
                    pr_review.occurred_at
                ));
            }
        }
    } else {
        output.push_str("No user data available.\n");
    }
    output
}

/// Build a markdown report from the activity data.
pub fn format_markdown(
    activity: &user_activity::ResponseData,
    start_date: ChronoDateTime<Utc>,
    end_date: ChronoDateTime<Utc>,
    username: &str,
) -> String {
    let mut output = String::new();
    if let Some(user) = &activity.user {
        let cc = &user.contributions_collection;
        output.push_str(&format!("# GitHub Activity Report for {}\n\n", username));
        output.push_str(&format!("**Time Period:** {} to {}\n\n", start_date.to_rfc3339(), end_date.to_rfc3339()));
        output.push_str("## Summary\n\n");
        output.push_str(&format!("- **Total Commit Contributions:** {}\n", cc.total_commit_contributions));
        output.push_str(&format!("- **Total Issue Contributions:** {}\n", cc.total_issue_contributions));
        output.push_str(&format!("- **Total Pull Request Contributions:** {}\n", cc.total_pull_request_contributions));
        output.push_str(&format!("- **Total Pull Request Review Contributions:** {}\n\n", cc.total_pull_request_review_contributions));
        
        output.push_str("## Repository Contributions\n\n");
        output.push_str("| Repository             | Total Contributions |\n");
        output.push_str("|------------------------|---------------------|\n");
        for repo_contrib in &cc.commit_contributions_by_repository {
            output.push_str(&format!(
                "| {:<22} | {:>19} |\n",
                repo_contrib.repository.name_with_owner,
                repo_contrib.contributions.total_count
            ));
        }
        output.push_str("\n## Issue Contributions\n\n");
        output.push_str("| Issue Number | Title | URL |\n");
        output.push_str("|--------------|-------|-----|\n");
        if let Some(issue_conn) = &user.contributions_collection.issue_contributions.nodes {
            for issue_node in issue_conn {
                output.push_str(&format!(
                    "| {} | {} | {} |\n",
                    issue_node.issue.number,
                    issue_node.issue.title,
                    issue_node.issue.url
                ));
            }
        }
        output.push_str("\n## Pull Request Contributions\n\n");
        output.push_str("| PR Number | Title | URL |\n");
        output.push_str("|-----------|-------|-----|\n");
        if let Some(pr_conn) = &user.contributions_collection.pull_request_contributions.nodes {
            for pr_node in pr_conn {
                output.push_str(&format!(
                    "| {} | {} | {} |\n",
                    pr_node.pull_request.number,
                    pr_node.pull_request.title,
                    pr_node.pull_request.url
                ));
            }
        }
        output.push_str("\n## Pull Request Review Contributions\n\n");
        output.push_str("| PR Number | Title | URL | Occurred At |\n");
        output.push_str("|-----------|-------|-----|-------------|\n");
        if let Some(pr_review_conn) = &user.contributions_collection.pull_request_review_contributions.nodes {
            for pr_review in pr_review_conn {
                output.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    pr_review.pull_request_review.pull_request.number,
                    pr_review.pull_request_review.pull_request.title,
                    pr_review.pull_request_review.pull_request.url,
                    pr_review.occurred_at
                ));
            }
        }
    } else {
        output.push_str("No user data available.\n");
    }
    output
}