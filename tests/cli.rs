mod common;

use common::Harness;
use predicates::prelude::*;
use predicates::str::contains;

const VALID_UNTIL: i64 = 4_102_444_800_000;

#[test]
fn no_credentials_prints_login_required() {
    let h = Harness::new();

    h.run(&[]).success().stdout("cclimit: login required\n");
}

#[test]
fn prints_all_limits_in_one_line() {
    let h = Harness::new();
    h.given_credentials(VALID_UNTIL);
    h.given_usage_fixture();

    h.run(&[])
        .success()
        .stdout("5h 11% 2h15m | 7d 38% 1d3h | fable 68% 1d3h\n");
}

#[test]
fn server_error_prints_nothing_and_fails() {
    let h = Harness::new();
    h.given_credentials(VALID_UNTIL);
    h.given_api_error(500);

    h.run(&[]).failure().stdout("");
}

#[test]
fn expired_token_prints_token_expired() {
    let h = Harness::new();
    h.given_credentials(1_760_000_000_000);

    h.run(&[])
        .success()
        .stdout("cclimit: token expired\n")
        .stderr("cclimit: access token expired, start Claude Code to refresh it\n");
}

#[test]
fn unauthorized_prints_login_required() {
    let h = Harness::new();
    h.given_credentials(VALID_UNTIL);
    h.given_api_error(401);

    h.run(&[]).success().stdout("cclimit: login required\n");
}

#[test]
fn unreadable_response_fails_without_echoing_body() {
    let h = Harness::new();
    h.given_credentials(VALID_UNTIL);
    h.given_api_body(200, br#"{"limits": "test-token"}"#);

    h.run(&[])
        .failure()
        .stdout("")
        .stderr(contains("test-token").not());
}

#[test]
fn doctor_reports_sources_without_tokens() {
    let h = Harness::new();
    h.given_credentials(VALID_UNTIL);
    h.given_usage_fixture();

    h.run(&["doctor"]).success().stdout(
        contains("credentials: env")
            .and(contains("api: HTTP 200, limits 5h, 7d, fable"))
            .and(contains("test-token").not())
            .and(contains("test-refresh").not()),
    );
}

#[test]
fn doctor_with_expired_token_skips_the_api() {
    let h = Harness::new();
    h.given_credentials(1_760_000_000_000);

    h.run(&["doctor"])
        .success()
        .stdout(
            contains("credentials: env, expired ")
                .and(contains("api: skipped, access token expired")),
        )
        .stderr("");
}
