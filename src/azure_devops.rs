use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug)]
pub struct PullRequest {
    pub id: u32,
    pub title: String,
    /// Raw Azure DevOps status: "active", "completed", or "abandoned"
    pub status: String,
    pub html_url: String,
    pub draft: bool,
}

// Structs for az CLI JSON output
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AzPrResponse {
    pull_request_id: u32,
    title: String,
    status: String,
    #[serde(default)]
    is_draft: bool,
    source_ref_name: String,
}

/// Azure DevOps PR integration via the `az` CLI (requires the azure-devops
/// extension). Auth is delegated to `az login` / `az devops login` (PAT).
pub struct AzureDevOpsClient {
    organization: String,
    project: String,
}

impl AzureDevOpsClient {
    pub fn new(organization: String, project: String) -> Self {
        Self { organization, project }
    }

    fn organization_url(&self) -> String {
        format!("https://dev.azure.com/{}", self.organization)
    }

    pub fn has_auth(&self) -> bool {
        // A PAT via the extension's env var works without az login
        if std::env::var("AZURE_DEVOPS_EXT_PAT")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            return true;
        }
        std::process::Command::new("az")
            .args(["account", "show", "--output", "none"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn run_pr_list(&self, extra_args: &[&str]) -> Result<Vec<AzPrResponse>> {
        let organization_url = self.organization_url();
        let mut args = vec![
            "repos",
            "pr",
            "list",
            "--organization",
            &organization_url,
            "--project",
            &self.project,
            "--output",
            "json",
        ];
        args.extend_from_slice(extra_args);

        let output = std::process::Command::new("az")
            .args(&args)
            .output()
            .map_err(|e| Error::provider(format!("Failed to execute az command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("az login") || stderr.contains("authentication") || stderr.contains("TF400813") {
                return Err(Error::auth(
                    "Azure DevOps authentication failed. Run 'az login' (or 'az devops login') to authenticate.",
                ));
            }
            return Err(Error::provider(format!("Failed to fetch pull requests: {}", stderr)));
        }

        let stdout = String::from_utf8(output.stdout)?;
        if stdout.trim().is_empty() {
            return Ok(vec![]);
        }

        serde_json::from_str(&stdout)
            .map_err(|e| Error::provider(format!("Failed to parse pull requests from az output: {}", e)))
    }

    fn to_pull_request(&self, repo: &str, pr: AzPrResponse) -> (PullRequest, String) {
        let branch = pr
            .source_ref_name
            .strip_prefix("refs/heads/")
            .unwrap_or(&pr.source_ref_name)
            .to_string();
        let pull_request = PullRequest {
            id: pr.pull_request_id,
            title: pr.title,
            status: pr.status,
            html_url: self.pr_web_url(repo, pr.pull_request_id),
            draft: pr.is_draft,
        };
        (pull_request, branch)
    }

    pub fn get_pull_requests(&self, repo: &str, branch: &str) -> Result<Vec<PullRequest>> {
        let prs = self.run_pr_list(&["--repository", repo, "--source-branch", branch, "--status", "all"])?;
        Ok(prs.into_iter().map(|pr| self.to_pull_request(repo, pr).0).collect())
    }

    pub fn get_all_pull_requests(&self, repo: &str) -> Result<Vec<(PullRequest, String)>> {
        let prs = self.run_pr_list(&["--repository", repo, "--status", "active", "--top", "100"])?;
        Ok(prs.into_iter().map(|pr| self.to_pull_request(repo, pr)).collect())
    }

    /// Verify the az CLI, azure-devops extension, and authentication in one call
    pub fn test_connection(&self, repo: &str) -> Result<()> {
        self.run_pr_list(&["--repository", repo, "--top", "1"])?;
        Ok(())
    }

    fn pr_web_url(&self, repo: &str, id: u32) -> String {
        format!(
            "https://dev.azure.com/{}/{}/_git/{}/pullrequest/{}",
            percent_encode_segment(&self.organization),
            percent_encode_segment(&self.project),
            percent_encode_segment(repo),
            id
        )
    }

    /// Parse an Azure DevOps remote URL into (organization, project, repo).
    ///
    /// Supported forms:
    ///   git@ssh.dev.azure.com:v3/<org>/<project>/<repo>
    ///   ssh://git@ssh.dev.azure.com/v3/<org>/<project>/<repo>
    ///   https://[user@]dev.azure.com/<org>/<project>/_git/<repo>
    ///   https://<org>.visualstudio.com/[DefaultCollection/]<project>/_git/<repo>
    ///
    /// Percent-encoded segments (e.g. "My%20Project") are decoded.
    pub fn parse_azure_url(url: &str) -> Option<(String, String, String)> {
        let url = url.trim_end_matches(".git");

        // SSH forms: v3/<org>/<project>/<repo>
        for prefix in [
            "git@ssh.dev.azure.com:",
            "ssh://git@ssh.dev.azure.com/",
            "ssh://git@ssh.dev.azure.com:22/",
        ] {
            if let Some(rest) = url.strip_prefix(prefix) {
                let rest = rest.strip_prefix("v3/")?;
                let parts: Vec<&str> = rest.split('/').collect();
                if parts.len() == 3 {
                    return Some((
                        percent_decode(parts[0]),
                        percent_decode(parts[1]),
                        percent_decode(parts[2]),
                    ));
                }
                return None;
            }
        }

        // HTTPS form: https://[user@]dev.azure.com/<org>/<project>/_git/<repo>
        if let Some(rest) = url.strip_prefix("https://") {
            let rest = match rest.split_once('@') {
                Some((_user, host_and_path)) => host_and_path,
                None => rest,
            };
            if let Some(path) = rest.strip_prefix("dev.azure.com/") {
                let parts: Vec<&str> = path.split('/').collect();
                if parts.len() == 4 && parts[2] == "_git" {
                    return Some((
                        percent_decode(parts[0]),
                        percent_decode(parts[1]),
                        percent_decode(parts[3]),
                    ));
                }
                return None;
            }
            // Legacy form: https://<org>.visualstudio.com/[DefaultCollection/]<project>/_git/<repo>
            if let Some((host, path)) = rest.split_once('/') {
                if let Some(org) = host.strip_suffix(".visualstudio.com") {
                    let path = path.strip_prefix("DefaultCollection/").unwrap_or(path);
                    let parts: Vec<&str> = path.split('/').collect();
                    if parts.len() == 3 && parts[1] == "_git" {
                        return Some((percent_decode(org), percent_decode(parts[0]), percent_decode(parts[2])));
                    }
                }
            }
        }

        None
    }
}

/// Web URL of the repository's pull request list
pub fn pr_list_web_url(organization: &str, project: &str, repo: &str) -> String {
    format!(
        "https://dev.azure.com/{}/{}/_git/{}/pullrequests",
        percent_encode_segment(organization),
        percent_encode_segment(project),
        percent_encode_segment(repo)
    )
}

/// Decode %XX escapes (and nothing else) in a URL path segment
fn percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&segment[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| segment.to_string())
}

/// Encode a string for use as a URL path segment (RFC 3986 unreserved kept)
fn percent_encode_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_azure_url() {
        let test_cases = vec![
            (
                "git@ssh.dev.azure.com:v3/myorg/MyProject/my-repo",
                Some(("myorg".to_string(), "MyProject".to_string(), "my-repo".to_string())),
            ),
            (
                "git@ssh.dev.azure.com:v3/myorg/My%20Project/my-repo",
                Some(("myorg".to_string(), "My Project".to_string(), "my-repo".to_string())),
            ),
            (
                "ssh://git@ssh.dev.azure.com/v3/myorg/MyProject/my-repo",
                Some(("myorg".to_string(), "MyProject".to_string(), "my-repo".to_string())),
            ),
            (
                "https://dev.azure.com/myorg/MyProject/_git/my-repo",
                Some(("myorg".to_string(), "MyProject".to_string(), "my-repo".to_string())),
            ),
            (
                "https://user@dev.azure.com/myorg/My%20Project/_git/my-repo",
                Some(("myorg".to_string(), "My Project".to_string(), "my-repo".to_string())),
            ),
            (
                "https://myorg.visualstudio.com/MyProject/_git/my-repo",
                Some(("myorg".to_string(), "MyProject".to_string(), "my-repo".to_string())),
            ),
            (
                "https://myorg.visualstudio.com/DefaultCollection/MyProject/_git/my-repo",
                Some(("myorg".to_string(), "MyProject".to_string(), "my-repo".to_string())),
            ),
            ("https://github.com/owner/repo.git", None),
            ("git@github.com:owner/repo.git", None),
            ("https://dev.azure.com/myorg/onlyproject", None),
        ];

        for (url, expected) in test_cases {
            assert_eq!(AzureDevOpsClient::parse_azure_url(url), expected, "url: {}", url);
        }
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("My%20Project"), "My Project");
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("50%"), "50%");
        assert_eq!(percent_decode("MinunMaatilani%20-%20Plant"), "MinunMaatilani - Plant");
    }

    #[test]
    fn test_pr_web_url() {
        let client = AzureDevOpsClient::new("myorg".to_string(), "My Project".to_string());
        assert_eq!(
            client.pr_web_url("my-repo", 42),
            "https://dev.azure.com/myorg/My%20Project/_git/my-repo/pullrequest/42"
        );
    }
}
