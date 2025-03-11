use chrono::Duration;
use clap::Parser;
use regex::Regex;
use std::str::FromStr;

/// Command-line arguments for the GitHub activity tool.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// GitHub username (allowed: letters, digits, hyphens; max 39 characters)
    #[arg(short, long)]
    pub username: GitHubUsername,

    /// Time period (day, week, month)
    #[arg(short, long, value_parser = parse_period)]
    pub period: Duration,
}

/// A newtype representing a GitHub username with validation.
#[derive(Debug, Clone)]
pub struct GitHubUsername(pub String);

impl FromStr for GitHubUsername {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("Username cannot be empty".into());
        }
        if s.len() > 39 {
            return Err("Username cannot be longer than 39 characters".into());
        }
        // GitHub usernames can contain letters, digits, and hyphens,
        // and cannot start or end with a hyphen.
        let re = Regex::new(r"^[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?$")
            .map_err(|e| format!("Regex error: {}", e))?;
        if !re.is_match(s) {
            return Err("Username contains invalid characters. Allowed: letters, digits, and hyphens (but not at the beginning or end)".into());
        }
        Ok(GitHubUsername(s.to_string()))
    }
}

impl std::fmt::Display for GitHubUsername {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Parses a time period string ("day", "week", or "month") into a `chrono::Duration`.
pub fn parse_period(arg: &str) -> Result<Duration, String> {
    match arg.to_lowercase().as_str() {
        "day" => Ok(Duration::days(1)),
        "week" => Ok(Duration::weeks(1)),
        "month" => Ok(Duration::days(30)),
        _ => Err(format!("Invalid period: {}. Use 'day', 'week', or 'month'", arg)),
    }
}