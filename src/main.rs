mod api;
mod cli;
mod config;
mod credentials;
mod render;

use anyhow::Result;
use api::Limits;
use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use credentials::Credentials;
use std::process::ExitCode;

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("cclimit: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env()?;
    match cli.command {
        None => run(&config),
        Some(Command::Doctor) => doctor(&config),
    }
}

enum Failure {
    TokenExpired,
    LoginRequired(anyhow::Error),
    Transient(anyhow::Error),
}

fn run(config: &Config) -> Result<()> {
    let limits = credentials::load(config.credentials_file.as_deref())
        .map_err(Failure::LoginRequired)
        .and_then(|(_, credentials)| fetch_limits(config, &credentials));
    match limits {
        Ok(limits) => println!("{}", render::render(&limits, config.now)),
        Err(Failure::TokenExpired) => {
            eprintln!("cclimit: access token expired, start Claude Code to refresh it");
            println!("cclimit: token expired");
        }
        Err(Failure::LoginRequired(e)) => {
            eprintln!("cclimit: {e:#}");
            println!("cclimit: login required");
        }
        Err(Failure::Transient(e)) => return Err(e),
    }
    Ok(())
}

fn fetch_limits(config: &Config, credentials: &Credentials) -> Result<Limits, Failure> {
    if credentials.is_expired(config.now) {
        return Err(Failure::TokenExpired);
    }
    api::fetch(&config.api_base, credentials.access_token()).map_err(|e| match e {
        api::Error::Unauthorized(_) => Failure::LoginRequired(e.into()),
        _ => Failure::Transient(e.into()),
    })
}

fn doctor(config: &Config) -> Result<()> {
    let now = config.now;
    let credentials = match credentials::load(config.credentials_file.as_deref()) {
        Ok((source, credentials)) => {
            let expiry = match credentials.expires_at() {
                Some(t) if credentials.is_expired(now) => {
                    format!("expired {} ago", render::between(t, now))
                }
                Some(t) => format!("expires in {}", render::between(now, t)),
                None => "expiry unknown".to_string(),
            };
            println!("credentials: {source}, {expiry}");
            credentials
        }
        Err(e) => {
            println!("credentials: {e:#}");
            println!("api: skipped, no credentials");
            return Ok(());
        }
    };
    let api = match fetch_limits(config, &credentials) {
        Ok(limits) => {
            let labels: Vec<_> = limits.iter().map(|l| l.label()).collect();
            format!("HTTP 200, limits {}", labels.join(", "))
        }
        Err(Failure::TokenExpired) => "skipped, access token expired".to_string(),
        Err(Failure::LoginRequired(e) | Failure::Transient(e)) => format!("{e:#}"),
    };
    println!("api: {api}");
    Ok(())
}
