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
          output.push_str(&format!("- {}: {}\n", repo_contrib.repository.name_with_owner, repo_contrib.contributions.total_count));
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
      output.push_str("| Repository | Total Contributions |\n");
      output.push_str("|------------|---------------------|\n");
      for repo_contrib in &cc.commit_contributions_by_repository {
          output.push_str(&format!("| {} | {} |\n", repo_contrib.repository.name_with_owner, repo_contrib.contributions.total_count));
      }
  } else {
      output.push_str("No user data available.\n");
  }
  output
}