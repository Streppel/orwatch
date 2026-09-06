mod api;
mod config;
mod render;
mod secrets;

use anyhow::Context;
use api::KeyResponse;
use clap::{Parser, Subcommand};
use config::Config;
use secrets::KeyError;

#[derive(Parser)]
#[command(
    name = "orwatch",
    about = "OpenRouter spend watcher (CLI + waybar)",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Ignore the on-disk cache and hit the API.
    #[arg(long, global = true)]
    fresh: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Human-readable report (default).
    Status,
    /// JSON for a waybar custom module. Always exits 0.
    Waybar,
    /// Raw snapshot JSON.
    Json,
}

fn main() {
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Cmd::Status) {
        Cmd::Waybar => {
            let out = waybar_payload(cli.fresh);
            println!("{}", serde_json::to_string(&out).expect("waybar json"));
        }
        Cmd::Status => {
            if let Err(err) = run_status(cli.fresh) {
                eprintln!("orwatch: {err}");
                std::process::exit(1);
            }
        }
        Cmd::Json => {
            if let Err(err) = run_json(cli.fresh) {
                eprintln!("orwatch: {err}");
                std::process::exit(1);
            }
        }
    }
}

fn snapshot(fresh: bool, allow_stale: bool) -> Result<KeyResponse, anyhow::Error> {
    let key = secrets::load_api_key()?;
    let cfg = Config::load();
    let ttl = if fresh {
        std::time::Duration::ZERO
    } else {
        cfg.cache_ttl()
    };
    api::fetch_or_cache(&key, &config::cache_path(), ttl, allow_stale)
        .context("fetching OpenRouter key usage")
}

fn run_status(fresh: bool) -> Result<(), anyhow::Error> {
    let snap = snapshot(fresh, false)?;
    println!("{}", render::human(&snap.data));
    Ok(())
}

fn run_json(fresh: bool) -> Result<(), anyhow::Error> {
    let snap = snapshot(fresh, false)?;
    println!("{}", serde_json::to_string_pretty(&snap)?);
    Ok(())
}

fn waybar_payload(fresh: bool) -> render::WaybarOut {
    match snapshot(fresh, true) {
        Ok(snap) => render::waybar_ok(&snap.data, &Config::load()),
        Err(err) => {
            let msg = match err.downcast_ref::<KeyError>() {
                Some(KeyError::Missing) => {
                    "OPENROUTER_API_KEY missing.\nUncomment it in ~/.secrets"
                }
                Some(KeyError::Placeholder(_)) => {
                    "OPENROUTER_API_KEY is still the placeholder in ~/.secrets"
                }
                None => {
                    return render::waybar_err(&format!("{err:#}"));
                }
            };
            render::waybar_err(msg)
        }
    }
}
