//! Project links shown on the login screen. Empty = not configured yet: the
//! button is shown disabled and nothing is fetched. The README header carries
//! the same Discord and website links: change them there too.

/// Source code repository (GitHub page).
pub const GITHUB: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer";
/// Waters Roux bug-report chooser (GitHub issue forms); used by Report a Bug.
pub const NEW_ISSUE: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer/issues/new/choose";
/// Resident feature requests and community feedback (GitHub Issues).
pub const FEATURE_REQUESTS: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer/issues";
/// Official viewer downloads and release notes (not a pre-release distribution approval).
pub const RELEASES: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer/releases";
/// Project documentation; do not send Waters Roux users to the Aurora wiki.
pub const WIKI: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer/tree/main/docs";
/// Community Discord invite.
pub const DISCORD: &str = "";
/// Temporary project home, until a dedicated Waters Roux website exists.
pub const WEBSITE: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer";
/// Temporary notice that Waters Roux privacy documentation needs release review.
/// Never represent Aurora's privacy policy as Waters Roux's own.
pub const PRIVACY_POLICY: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer/blob/main/docs/WATERS_ROUX_PRIVACY_STATUS.md";
/// News feed: this folder of `main`, hand-edited Markdown files, one
/// `<language>.md` per UI language (`news/` at the repository root). The
/// viewer keeps what it read in its cache as a JSON array of
/// `{"date", "title", "summary", "url"}`.
pub const NEWS_FEED: &str = "https://raw.githubusercontent.com/WynnieOS/Waters-Roux-Viewer/feat/waters-roux-branded-beta/news/";
/// Files of the source repository as GitHub serves them unrendered; a branch
/// name and a path follow.
pub const RAW_FILES: &str = "https://raw.githubusercontent.com/WynnieOS/Waters-Roux-Viewer/";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_support_links_do_not_route_residents_to_aurora() {
        let root = "https://github.com/WynnieOS/Waters-Roux-Viewer";
        assert_eq!(WEBSITE, root);
        assert_eq!(GITHUB, root);
        assert_eq!(NEW_ISSUE, format!("{root}/issues/new/choose"));
        assert_eq!(FEATURE_REQUESTS, format!("{root}/issues"));
        assert_eq!(RELEASES, format!("{root}/releases"));
        for link in [WEBSITE, GITHUB, NEW_ISSUE, FEATURE_REQUESTS, RELEASES, WIKI, PRIVACY_POLICY] {
            assert!(link.starts_with(root), "Waters Roux link unexpectedly leaves the repo: {link}");
        }
    }
}
