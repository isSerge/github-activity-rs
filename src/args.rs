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

    /// Output format: plain, markdown, or json
    #[arg(short, long, default_value = "json", value_parser = parse_output_format)]
    pub format: OutputFormat,
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
            .ok_or_else(|| "Invalid period format. Use e.g., 1d, 7d, 30d, 2w, 1m")?,
    );

    let amount: i64 = amount.parse().map_err(|_| "Invalid number in period")?;

    match unit {
        "d" => Ok(Duration::days(amount)),
        "w" => Ok(Duration::weeks(amount)),
        "m" => Ok(Duration::days(amount * 30)),
        _ => Err(format!(
            "Invalid period unit: {}. Use d (days), w (weeks), or m (months)",
            unit
        )),
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
            naive_date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| "Invalid time conversion".to_string())?,
            Utc,
        ));
    }

    Err(format!(
        "Invalid date format. Use ISO 8601 format (e.g., 2024-01-01 or 2024-01-01T00:00:00Z)"
    ))
}

/// Supported output formats.
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Plain,
    Markdown,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" => Ok(OutputFormat::Plain),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!(
                "Invalid output format: {}. Use plain, markdown, or json",
                s
            )),
        }
    }
}

/// A helper to use the FromStr implementation.
fn parse_output_format(s: &str) -> Result<OutputFormat, String> {
    s.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_valid_github_username() {
        let valid = "valid-user123";
        let username = valid.parse::<GitHubUsername>();
        assert!(username.is_ok());
        assert_eq!(username.unwrap().0, valid);
    }

    #[test]
    fn test_invalid_github_username_empty() {
        let invalid = "";
        let username = invalid.parse::<GitHubUsername>();
        assert!(username.is_err());
    }

    #[test]
    fn test_invalid_github_username_too_long() {
        let invalid = "a".repeat(40);
        let username = invalid.parse::<GitHubUsername>();
        assert!(username.is_err());
    }

    #[test]
    fn test_invalid_github_username_invalid_chars() {
        let invalid = "invalid_username!";
        let username = invalid.parse::<GitHubUsername>();
        assert!(username.is_err());
    }

    #[test]
    fn test_parse_period_valid_days() {
        let period = parse_period("7d");
        assert!(period.is_ok());
        let duration = period.unwrap();
        assert_eq!(duration.num_days(), 7);
    }

    #[test]
    fn test_parse_period_valid_weeks() {
        let period = parse_period("2w");
        assert!(period.is_ok());
        let duration = period.unwrap();
        assert_eq!(duration.num_days(), 14);
    }

    #[test]
    fn test_parse_period_valid_months() {
        let period = parse_period("1m");
        assert!(period.is_ok());
        let duration = period.unwrap();
        // Assuming one month is interpreted as 30 days.
        assert_eq!(duration.num_days(), 30);
    }

    #[test]
    fn test_parse_period_invalid_format() {
        let period = parse_period("abc");
        assert!(period.is_err());
    }

    #[test]
    fn test_parse_period_invalid_unit() {
        let period = parse_period("10y");
        assert!(period.is_err());
    }

    #[test]
    fn test_parse_datetime_rfc3339() {
        let dt_str = "2024-01-01T12:34:56Z";
        let dt = parse_datetime(dt_str).expect("Should parse successfully");
        // Format using rfc3339 options that enforce the Z suffix for UTC.
        let formatted = dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert_eq!(formatted, dt_str);
    }

    #[test]
    fn test_parse_datetime_simple_date() {
        let dt_str = "2024-01-01";
        let dt = parse_datetime(dt_str);
        assert!(dt.is_ok());
        let dt = dt.unwrap();
        // Expect midnight UTC.
        assert_eq!(dt.to_rfc3339(), "2024-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let dt_str = "not a date";
        let dt = parse_datetime(dt_str);
        assert!(dt.is_err());
    }

    #[test]
    fn test_get_date_range_period() {
        // When period is provided, from/to should be computed relative to now.
        let period = Some(chrono::Duration::days(7));
        let args = Args {
            username: "dummy".parse().unwrap(),
            period,
            from: None,
            to: None,
            repo: None,
            org: None,
            format: OutputFormat::Json,
        };
        let range = args.get_date_range();
        assert!(range.is_ok());
        let (start, end) = range.unwrap();
        // The difference should be 7 days (or very close, depending on execution time).
        assert_eq!((end - start).num_days(), 7);
    }

    #[test]
    fn test_get_date_range_from_to_valid() {
        let from = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let args = Args {
            username: "dummy".parse().unwrap(),
            period: None,
            from: Some(from),
            to: Some(to),
            repo: None,
            org: None,
            format: OutputFormat::Json,
        };
        let range = args.get_date_range();
        assert!(range.is_ok());
        let (s, e) = range.unwrap();
        assert_eq!(s, from);
        assert_eq!(e, to);
    }

    #[test]
    fn test_get_date_range_invalid() {
        // Test error when start date is not before end date.
        let from = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let args = Args {
            username: "dummy".parse().unwrap(),
            period: None,
            from: Some(from),
            to: Some(to),
            repo: None,
            org: None,
            format: OutputFormat::Json,
        };
        let range = args.get_date_range();
        assert!(range.is_err());
    }

    #[test]
    fn test_output_format_from_str_valid() {
        let json: Result<OutputFormat, _> = "json".parse();
        let markdown: Result<OutputFormat, _> = "markdown".parse();
        let plain: Result<OutputFormat, _> = "plain".parse();
        assert!(json.is_ok());
        assert!(markdown.is_ok());
        assert!(plain.is_ok());
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        let invalid: Result<OutputFormat, _> = "invalid".parse();
        assert!(invalid.is_err());
    }
}
