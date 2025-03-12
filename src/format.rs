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
        output.push_str(&format!("  Total Contributions: {}\n", cc.contribution_calendar.total_contributions));
        for week in &cc.contribution_calendar.weeks {
            for day in &week.contribution_days {
                output.push_str(&format!("    {}: {} contributions (weekday {})\n", day.date, day.contribution_count, day.weekday));
            }
        }
        output.push_str("\n");

        // Repository Contributions
        output.push_str("Repository Contributions:\n");
        for repo_contrib in &cc.commit_contributions_by_repository {
            output.push_str(&format!(
                "- {}: {} commits\n",
                repo_contrib.repository.name_with_owner,
                repo_contrib.contributions.total_count
            ));
        }
        output.push_str("\n");

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
        output.push_str("\n");

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
        output.push_str("\n");

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

        // Contribution Calendar
        output.push_str("## Contribution Calendar\n\n");
        output.push_str(&format!("**Total Contributions:** {}\n\n", cc.contribution_calendar.total_contributions));
        for week in &cc.contribution_calendar.weeks {
            for day in &week.contribution_days {
                output.push_str(&format!("* {}: {} contributions (weekday {})\n", day.date, day.contribution_count, day.weekday));
            }
        }
        output.push_str("\n");

        // Repository Contributions
        output.push_str("## Repository Contributions\n\n");
        output.push_str("| Repository             | Commits |\n");
        output.push_str("|------------------------|---------|\n");
        for repo_contrib in &cc.commit_contributions_by_repository {
            output.push_str(&format!("| {:<22} | {:>7} |\n",
                repo_contrib.repository.name_with_owner,
                repo_contrib.contributions.total_count
            ));
        }
        output.push_str("\n");

        // Issue Contributions
        output.push_str("## Issue Contributions\n\n");
        output.push_str("| Issue # | Title | URL | Created At | State | Closed At |\n");
        output.push_str("|---------|-------|-----|------------|-------|-----------|\n");
        if let Some(nodes) = &cc.issue_contributions.nodes {
            for node in nodes {
                let issue = &node.issue;
                output.push_str(&format!("| {} | {} | {} | {} | {} | {} |\n",
                    issue.number,
                    issue.title,
                    issue.url,
                    issue.created_at,
                    issue.state,
                    issue.closed_at.as_deref().unwrap_or("N/A")
                ));
            }
        }
        output.push_str("\n");

        // Pull Request Contributions
        output.push_str("## Pull Request Contributions\n\n");
        output.push_str("| PR # | Title | URL | Created At | State | Merged | Merged At | Closed At |\n");
        output.push_str("|------|-------|-----|------------|-------|--------|-----------|-----------|\n");
        if let Some(nodes) = &cc.pull_request_contributions.nodes {
            for node in nodes {
                let pr = &node.pull_request;
                output.push_str(&format!("| {} | {} | {} | {} | {} | {} | {} | {} |\n",
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
        output.push_str("\n");

        // Pull Request Review Contributions
        output.push_str("## Pull Request Review Contributions\n\n");
        output.push_str("| PR # | Title | URL | Occurred At |\n");
        output.push_str("|------|-------|-----|-------------|\n");
        if let Some(nodes) = &cc.pull_request_review_contributions.nodes {
            for node in nodes {
                let pr_review = &node.pull_request_review;
                output.push_str(&format!("| {} | {} | {} | {} |\n",
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