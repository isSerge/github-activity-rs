use chrono::{DateTime, Duration, Utc};
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

    /// Time period (e.g., 1d, 7d, 30d, 2w, 1m, 3m)
    /// Mutually exclusive with --from and --to
    #[arg(short, long, value_parser = parse_period, conflicts_with_all = ["from", "to"])]
    pub period: Option<Duration>,

    /// Start date in ISO 8601 format (e.g., 2024-01-01 or 2024-01-01T00:00:00Z)
    /// Required if --to is specified
    #[arg(long, requires = "to", value_parser = parse_datetime)]
    pub from: Option<DateTime<Utc>>,

    /// End date in ISO 8601 format (e.g., 2024-03-01 or 2024-03-01T00:00:00Z)
    /// Required if --from is specified
    #[arg(long, requires = "from", value_parser = parse_datetime)]
    pub to: Option<DateTime<Utc>>,

    /// Optional repository filter in the format "owner/repo"
    #[arg(long)]
    pub repo: Option<String>,

    /// Optional organization filter (only contributions from repos in this organization)
    #[arg(long)]
    pub org: Option<String>,
}

impl Args {
    /// Get the date range for the query
    pub fn get_date_range(&self) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
        match (self.period, self.from, self.to) {
            (Some(period), None, None) => {
                let end = Utc::now();
                let start = end - period;
                Ok((start, end))
            }
            (None, Some(from), Some(to)) => {
                if from >= to {
                    return Err("Start date must be before end date".to_string());
                }
                Ok((from, to))
            }
            _ => Err("Either specify --period or both --from and --to".to_string()),
        }
    }
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

/// Parses a time period string into a `chrono::Duration`.
fn parse_period(arg: &str) -> Result<Duration, String> {
    let (amount, unit) = arg.split_at(
        arg.find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| "Invalid period format. Use e.g., 1d, 7d, 30d, 2w, 1m")?
    );

    let amount: i64 = amount.parse()
        .map_err(|_| "Invalid number in period")?;

    match unit {
        "d" => Ok(Duration::days(amount)),
        "w" => Ok(Duration::weeks(amount)),
        "m" => Ok(Duration::days(amount * 30)),
        _ => Err(format!("Invalid period unit: {}. Use d (days), w (weeks), or m (months)", unit)),
    }
}

/// Parses a datetime string in ISO 8601 format
fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    // Try parsing with different formats
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // For simple dates (YYYY-MM-DD), parse as midnight UTC
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
      return Ok(DateTime::<Utc>::from_naive_utc_and_offset(
          naive_date.and_hms_opt(0, 0, 0)
              .ok_or_else(|| "Invalid time conversion".to_string())?,
          Utc,
      ));
  }

    Err(format!("Invalid date format. Use ISO 8601 format (e.g., 2024-01-01 or 2024-01-01T00:00:00Z)"))
}