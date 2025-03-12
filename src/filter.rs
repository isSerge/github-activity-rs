use crate::github::user_activity;

/// Filters the activity data based on repository and organization filters.
///
/// - `repo_filter`: When provided, only contributions from the repository matching this value are retained.
/// - `org_filter`: When provided, only contributions from repositories whose name starts with "<org_filter>/" are retained.
pub fn filter_activity(
    mut activity: user_activity::ResponseData,
    repo_filter: &Option<String>,
    org_filter: &Option<String>,
) -> user_activity::ResponseData {
    if let Some(user) = activity.user.as_mut() {
        // Clone the list so we can filter it.
        let mut filtered_repos = user
            .contributions_collection
            .commit_contributions_by_repository
            .clone();

        if let Some(repo_filter) = repo_filter {
            filtered_repos
                .retain(|repo_contrib| repo_contrib.repository.name_with_owner == *repo_filter);
        }

        if let Some(org_filter) = org_filter {
            filtered_repos.retain(|repo_contrib| {
                repo_contrib
                    .repository
                    .name_with_owner
                    .starts_with(&format!("{}/", org_filter))
            });
        }

        // Update the user's contributions collection with the filtered list.
        user.contributions_collection
            .commit_contributions_by_repository = filtered_repos;
    }
    activity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::user_activity;

    // Helper to construct dummy ResponseData with multiple repository contributions.
    fn dummy_response_data_for_filtering() -> user_activity::ResponseData {
        let repo1 = user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepository {
            repository: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryRepository {
                name_with_owner: "org1/repo1".to_string(),
                updated_at: "2025-03-10T00:00:00Z".to_string(),
            },
            contributions: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryContributions {
                total_count: 10,
            },
        };
        let repo2 = user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepository {
            repository: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryRepository {
                name_with_owner: "org2/repo2".to_string(),
                updated_at: "2025-03-11T00:00:00Z".to_string(),
            },
            contributions: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryContributions {
                total_count: 5,
            },
        };
        let repo3 = user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepository {
            repository: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryRepository {
                name_with_owner: "org1/repo3".to_string(),
                updated_at: "2025-03-12T00:00:00Z".to_string(),
            },
            contributions: user_activity::UserActivityUserContributionsCollectionCommitContributionsByRepositoryContributions {
                total_count: 3,
            },
        };

        let contributions_collection = user_activity::UserActivityUserContributionsCollection {
            total_commit_contributions: 0,
            total_issue_contributions: 0,
            total_pull_request_contributions: 0,
            total_pull_request_review_contributions: 0,
            contribution_calendar: user_activity::UserActivityUserContributionsCollectionContributionCalendar {
                total_contributions: 0,
                weeks: vec![],
            },
            commit_contributions_by_repository: vec![repo1, repo2, repo3],
            issue_contributions: user_activity::UserActivityUserContributionsCollectionIssueContributions {
                total_count: 0,
                page_info: user_activity::UserActivityUserContributionsCollectionIssueContributionsPageInfo {
                    end_cursor: None,
                    has_next_page: false,
                },
                nodes: None,
            },
            pull_request_contributions: user_activity::UserActivityUserContributionsCollectionPullRequestContributions {
                total_count: 0,
                page_info: user_activity::UserActivityUserContributionsCollectionPullRequestContributionsPageInfo {
                    end_cursor: None,
                    has_next_page: false,
                },
                nodes: None,
            },
            pull_request_review_contributions: user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributions {
                total_count: 0,
                page_info: user_activity::UserActivityUserContributionsCollectionPullRequestReviewContributionsPageInfo {
                    end_cursor: None,
                    has_next_page: false,
                },
                nodes: None,
            },
        };

        user_activity::ResponseData {
            user: Some(user_activity::UserActivityUser {
                contributions_collection,
            }),
        }
    }

    #[test]
    fn test_filter_no_filter() {
        let data = dummy_response_data_for_filtering();
        let filtered = filter_activity(data.clone(), &None, &None);
        let repos = filtered
            .user
            .unwrap()
            .contributions_collection
            .commit_contributions_by_repository;
        assert_eq!(repos.len(), 3);
    }

    #[test]
    fn test_filter_repo_only() {
        let data = dummy_response_data_for_filtering();
        let repo_filter = Some("org1/repo1".to_string());
        let filtered = filter_activity(data, &repo_filter, &None);
        let repos = filtered
            .user
            .unwrap()
            .contributions_collection
            .commit_contributions_by_repository;
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repository.name_with_owner, "org1/repo1");
    }

    #[test]
    fn test_filter_org_only() {
        let data = dummy_response_data_for_filtering();
        let org_filter = Some("org1".to_string());
        let filtered = filter_activity(data, &None, &org_filter);
        let repos = filtered
            .user
            .unwrap()
            .contributions_collection
            .commit_contributions_by_repository;
        assert_eq!(repos.len(), 2);
        let names: Vec<_> = repos
            .into_iter()
            .map(|r| r.repository.name_with_owner)
            .collect();
        assert!(names.contains(&"org1/repo1".to_string()));
        assert!(names.contains(&"org1/repo3".to_string()));
    }

    #[test]
    fn test_filter_repo_and_org() {
        let data = dummy_response_data_for_filtering();
        let repo_filter = Some("org1/repo3".to_string());
        let org_filter = Some("org1".to_string());
        let filtered = filter_activity(data, &repo_filter, &org_filter);
        let repos = filtered
            .user
            .unwrap()
            .contributions_collection
            .commit_contributions_by_repository;
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repository.name_with_owner, "org1/repo3");
    }

    #[test]
    fn test_filter_conflicting_filters() {
        let data = dummy_response_data_for_filtering();
        let repo_filter = Some("org2/repo2".to_string());
        let org_filter = Some("org1".to_string());
        let filtered = filter_activity(data, &repo_filter, &org_filter);
        let repos = filtered
            .user
            .unwrap()
            .contributions_collection
            .commit_contributions_by_repository;
        assert_eq!(repos.len(), 0);
    }
}
