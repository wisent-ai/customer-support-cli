use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use customer_support_cli::{
    RoutingRule, SlaPolicy, Ticket, build_queue, build_timeline, compute_sla, normalize_ticket,
    parse_optional_date, route_ticket,
};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Parser)]
#[command(
    name = "customer-support",
    version,
    about = "Deterministic customer support triage and queue operations CLI",
    after_help = "All routing and SLA decisions are deterministic and printed as JSON."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Normalize {
        #[arg(long)]
        ticket: PathBuf,
    },
    Queue {
        #[arg(long)]
        tickets: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        include_resolved: bool,
    },
    Sla {
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        at: Option<String>,
    },
    Route {
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        rules: PathBuf,
    },
    Timeline {
        #[arg(long)]
        ticket: PathBuf,
    },
}

fn read_json<T: DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let input = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&input).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn output<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Normalize { ticket } => output(&normalize_ticket(read_json::<Ticket>(&ticket)?)?),
        Command::Queue {
            tickets,
            policy,
            at,
            include_resolved,
        } => {
            let at = parse_optional_date(at.as_deref(), "--at")?.unwrap_or_else(Utc::now);
            output(&build_queue(
                read_json::<Vec<Ticket>>(&tickets)?,
                &read_json::<SlaPolicy>(&policy)?,
                at,
                include_resolved,
            )?)
        }
        Command::Sla { ticket, policy, at } => {
            let ticket = normalize_ticket(read_json::<Ticket>(&ticket)?)?;
            let at = parse_optional_date(at.as_deref(), "--at")?.unwrap_or_else(Utc::now);
            output(&compute_sla(
                &ticket,
                &read_json::<SlaPolicy>(&policy)?,
                at,
            )?)
        }
        Command::Route { ticket, rules } => {
            let ticket = normalize_ticket(read_json::<Ticket>(&ticket)?)?;
            output(&route_ticket(
                &ticket,
                read_json::<Vec<RoutingRule>>(&rules)?,
            )?)
        }
        Command::Timeline { ticket } => {
            let ticket = normalize_ticket(read_json::<Ticket>(&ticket)?)?;
            output(&build_timeline(&ticket))
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
