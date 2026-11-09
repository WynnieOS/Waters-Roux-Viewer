//! Project links shown on the login screen. Empty = not configured yet: the
//! button is shown disabled and nothing is fetched. The README header carries
//! the same Discord and website links: change them there too.

/// Source code repository (GitHub page).
pub const GITHUB: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer";
/// Fork's pull request index (issues are currently disabled), opened by "Report a bug" (login
/// screen, top bar, Help menu) and by the crash window. Dedicated issue reporting is not yet configured.
pub const NEW_ISSUE: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer/pulls";
/// The project's wiki, generated from the guides of `docs/`
/// (`scripts/build-wiki.ps1`): Help › Aurora Viewer wiki.
pub const WIKI: &str = "https://github.com/Aurora-Viewer/Aurora-Viewer/wiki";
/// Community Discord invite.
pub const DISCORD: &str = "";
/// Temporary project home, until a dedicated Waters Roux website exists.
pub const WEBSITE: &str = "https://github.com/WynnieOS/Waters-Roux-Viewer";
/// The privacy policy on the website (Preferences › Privacy › Your data).
pub const PRIVACY_POLICY: &str = "https://aurora-viewer.org/privacy/";
/// News feed: this folder of `main`, hand-edited Markdown files, one
/// `<language>.md` per UI language (`news/` at the repository root). The
/// viewer keeps what it read in its cache as a JSON array of
/// `{"date", "title", "summary", "url"}`.
pub const NEWS_FEED: &str = "https://raw.githubusercontent.com/WynnieOS/Waters-Roux-Viewer/feat/waters-roux-branded-beta/news/";
/// Files of the source repository as GitHub serves them unrendered; a branch
/// name and a path follow.
pub const RAW_FILES: &str = "https://raw.githubusercontent.com/WynnieOS/Waters-Roux-Viewer/";
