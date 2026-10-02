use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use customer_support_cli::{
    RoutingRule, SlaPolicy, Ticket, build_queue, build_timeline, compute_sla, normalize_ticket,
    parse_optional_date, route_ticket,
};
use wisent_errors::cli_output::{output, read_json};

#[derive(Parser)]
#[command(
    name = "customer-support",
    version,
    about = "Deterministic customer support triage and queue operations CLI",
    after_help = "All routing and SLA decisions are deterministic. Every command prints JSON, or with --text one `path: value` line per field from the same data. Exit 2: the invocation is wrong; exit 1: an input could not be read or refused."
)]
struct Cli {
    /// Print one `path: value` line per field instead of JSON.
    #[arg(long, global = true)]
    text: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read one ticket and print it in canonical form.
    Normalize {
        /// The ticket JSON file.
        #[arg(long)]
        ticket: PathBuf,
    },
    /// Order open tickets by how close each is to breaching its SLA.
    Queue {
        /// A JSON array of tickets.
        #[arg(long)]
        tickets: PathBuf,
        /// The SLA policy JSON file.
        #[arg(long)]
        policy: PathBuf,
        /// The moment the queue is computed for (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
        /// Keep resolved tickets in the queue.
        #[arg(long)]
        include_resolved: bool,
    },
    /// Compute one ticket's SLA targets and whether they are met.
    Sla {
        /// The ticket JSON file.
        #[arg(long)]
        ticket: PathBuf,
        /// The SLA policy JSON file.
        #[arg(long)]
        policy: PathBuf,
        /// The moment the SLA is computed for (RFC 3339); now when omitted.
        #[arg(long)]
        at: Option<String>,
    },
    /// Pick the team a ticket goes to from ordered routing rules.
    Route {
        /// The ticket JSON file.
        #[arg(long)]
        ticket: PathBuf,
        /// A JSON array of routing rules, first match wins.
        #[arg(long)]
        rules: PathBuf,
    },
    /// List one ticket's events in time order.
    Timeline {
        /// The ticket JSON file.
        #[arg(long)]
        ticket: PathBuf,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let text = cli.text;
    match cli.command {
        Command::Normalize { ticket } => {
            output(&normalize_ticket(read_json::<Ticket>(&ticket)?)?, text)
        }
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
            )?, text)
        }
        Command::Sla { ticket, policy, at } => {
            let ticket = normalize_ticket(read_json::<Ticket>(&ticket)?)?;
            let at = parse_optional_date(at.as_deref(), "--at")?.unwrap_or_else(Utc::now);
            output(&compute_sla(
                &ticket,
                &read_json::<SlaPolicy>(&policy)?,
                at,
            )?, text)
        }
        Command::Route { ticket, rules } => {
            let ticket = normalize_ticket(read_json::<Ticket>(&ticket)?)?;
            output(&route_ticket(
                &ticket,
                read_json::<Vec<RoutingRule>>(&rules)?,
            )?, text)
        }
        Command::Timeline { ticket } => {
            let ticket = normalize_ticket(read_json::<Ticket>(&ticket)?)?;
            output(&build_timeline(&ticket), text)
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
