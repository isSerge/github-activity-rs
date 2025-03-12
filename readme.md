# GitHub Activity Reporter

GitHub Activity Reporter is a command-line tool written in Rust that fetches a GitHub user’s activity (commits, issues, pull requests, and pull request reviews) using the GitHub GraphQL API. The tool aggregates both base and paginated data, then formats the results as JSON, plain text, or Markdown. It also allows filtering contributions by repository or organization.

## Features

- Fetch GitHub Contributions: Retrieves commits, issue contributions, pull requests, and pull request reviews.
- Multiple Output Formats: Display results as JSON, plain text, or Markdown reports.
- Filtering Capabilities: Filter contributions by specific repositories or organizations.
- Configurable Date Ranges: Specify time periods either as a relative duration (e.g., 7d for 7 days) or using ISO 8601 start and end dates.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)
- A valid GitHub Personal Access Token with permissions to read user contributions

## Installation

1. 	Clone the repository:

```sh
git clone https://github.com/yourusername/github-activity-rs.git
cd github-activity-rs
```

2. Build the project:
```sh
cargo build --release
```

3. Set up your environment:
Create `.env` file in the project root with at least the following:
```sh
GITHUB_TOKEN=your_github_token_here
```
You can optionally override the GraphQL URL if needed:
```sh
GITHUB_GRAPHQL_URL=https://api.github.com/graphql
```

## Usage
Run the application using cargo run with the appropriate arguments. For example:
- Using a time period (e.g., last 7 days):
```sh
cargo run -- --username octocat --period 7d --format markdown
```

- Using explicit start and end dates:
```sh
cargo run -- --username octocat --from 2024-01-01T00:00:00Z --to 2024-01-31T00:00:00Z --format plain
```

- Applying filters:
```
cargo run -- --username octocat --period 30d --repo "owner/repo" --org "owner" --format json
```

### Available command-line arguments:
- `--username`: GitHub username
- `--period`: Relative time period (e.g., 7d, 2w, 1m)
- `--from` and `--to`: ISO 8601 formatted start and end dates (mutually exclusive with `--period`)
- `--repo`: Filter results to contributions from the specified repository
- `--org`: Filter results to contributions from repositories in the specified organization
- `--format`: Output format (plain, markdown, or json)

## Testing
Run all tests using Cargo:
```sh
cargo test
```

## Furthere improvements

- Scheduled report generation
- Saving and processing historical data to identify long-term trends
- Multiple users and organization support


## License

This project is licensed under the [MIT License](LICENSE)
