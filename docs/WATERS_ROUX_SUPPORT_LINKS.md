# Waters Roux Beta 28 V3 — official links and support routing

**Owner requirement (2026-11-16):** Every *official Waters Roux product button and help action* must stay within the WynnieOS/Waters-Roux-Viewer project. Do not send residents to Aurora's website, issue tracker, downloads, feedback board, or GitHub App. Aurora attribution, technical history and legal notices remain intact.

| Product action | Final destination |
| --- | --- |
| Website / Project | https://github.com/WynnieOS/Waters-Roux-Viewer |
| Report a Bug | https://github.com/WynnieOS/Waters-Roux-Viewer/issues/new/choose |
| Download / Updates | https://github.com/WynnieOS/Waters-Roux-Viewer/releases |
| Feature Requests | https://github.com/WynnieOS/Waters-Roux-Viewer/issues |
| Documentation / Help | https://github.com/WynnieOS/Waters-Roux-Viewer/tree/main/docs |

## Technical findings

- GitHub repository metadata's Website field was `https://aurora-viewer.org` when audited. **Repository Settings must be changed by an administrator** to the official temporary homepage above. Updating README or Rust constants does not update repository metadata.
- GitHub `has_issues` was **true** at the last check. The default branch has an issue form, but its description referred to Aurora and `.github/ISSUE_TEMPLATE/config.yml` pointed feature requests to Aurora feedback. Both require correction.
- `crates/aurora-viewer/src/links.rs::NEW_ISSUE` pointed to Waters Roux **pull requests**, not bug reports. `WIKI` and `PRIVACY_POLICY` went to Aurora.
- The in-viewer bug-report and separate crash-report modules contained Aurora's GitHub API URLs and an embedded Aurora GitHub App client ID. V3 routes browser forms and issue APIs to Waters Roux while disabling direct OAuth until a dedicated, approved Waters Roux GitHub App is configured. **Never reuse Aurora's client ID.**
- The update feed `crates/aurora-updater/src/feed.rs` already targets Waters Roux releases; do not replace or break this working integration.
- The updater's existing internal policy filename and underlying source package names may contain `aurora` for compatibility; audit them separately before changing persistence formats. Do not remove required legal credits.
- The user's requested public website is **not yet built**. A GitHub Pages site may be planned later; until then, repository homepage is the temporary website.

## V3 acceptance (all pending until verified)

- [ ] In-viewer Website (About/login/help) opens the actual Waters Roux project.
- [ ] Report a Bug opens a working Waters Roux issue form, including crash-report browser fallback.
- [ ] Download / Updates goes to Waters Roux releases; no unapproved V2 artifact is presented as a stable release.
- [ ] Feature Requests opens Waters Roux Issues and the new feature-request form is selectable.
- [ ] Help/Wiki opens Waters Roux documentation, not Aurora's wiki.
- [ ] No Waters Roux product link goes to Aurora's website, bugs, feedback or downloads.
- [ ] Dedicated GitHub App integration is disabled until the authorized Waters Roux app is configured, security-reviewed, and tested.
- [ ] Interim privacy notice is reachable; replace with reviewed policy **before public v1.0.0**.
- [ ] Repository Website metadata and description are corrected by repo administrator.
- [ ] Windows build/CI tests pass; buttons checked in actual viewer.
- [ ] Third-party attribution and license notices remain intact.

This checklist covers a **source-development PR**, not a shipped Beta V3 executable.
