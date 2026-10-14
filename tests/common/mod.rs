use assert_cmd::Command;
use httpmock::MockServer;
use tempfile::TempDir;

const NOW: &str = "2026-10-13T19:14:59Z";

pub struct Harness {
    server: MockServer,
    dir: TempDir,
}

impl Harness {
    pub fn new() -> Self {
        Self {
            server: MockServer::start(),
            dir: TempDir::new().unwrap(),
        }
    }

    pub fn given_usage_fixture(&self) {
        self.given_api_body(200, include_bytes!("../fixtures/usage.json"));
    }

    pub fn given_api_error(&self, status: u16) {
        self.given_api_body(status, b"");
    }

    pub fn given_api_body(&self, status: u16, body: &[u8]) {
        self.server.mock(|when, then| {
            when.method("GET").path("/api/oauth/usage");
            then.status(status)
                .header("Content-Type", "application/json")
                .body(body);
        });
    }

    pub fn given_credentials(&self, expires_at_ms: i64) {
        let json = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "test-token",
                "refreshToken": "test-refresh",
                "expiresAt": expires_at_ms,
                "scopes": ["user:inference"],
                "subscriptionType": "max"
            }
        });
        std::fs::write(self.credentials_path(), json.to_string()).unwrap();
    }

    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        Command::cargo_bin("cclimit")
            .unwrap()
            .args(args)
            .env("CCLIMIT_API_BASE", self.server.base_url())
            .env("CCLIMIT_CREDENTIALS_FILE", self.credentials_path())
            .env("CCLIMIT_NOW", NOW)
            .assert()
    }

    fn credentials_path(&self) -> std::path::PathBuf {
        self.dir.path().join("credentials.json")
    }
}
