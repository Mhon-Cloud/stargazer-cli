//! Every command, flag and help text (clap derive). `--help` is what people and AI agents read
//! to learn the CLI: keep each line short and say what the command does, not how.
//!
//! One file per resource group, `stargazer <group> <verb>`, matching the console's menu:
//! account.rs (account, billing, inbox, usage), projects.rs (projects, zones), servers.rs,
//! containers.rs, network.rs (DNS, public IPs; VPCs go here), security.rs (SSH keys, security
//! groups, backups). A new group = a file here + a `Command` variant + a module in `commands/`
//! (see AGENTS.md, "Adding a command").

mod account;
mod containers;
mod network;
mod projects;
mod security;
mod servers;

use clap::{Args, Parser, Subcommand};

pub use account::*;
pub use containers::*;
pub use network::*;
pub use projects::*;
pub use security::*;
pub use servers::*;

use crate::output::OutputArg;

/// The console page where keys are made; said in every "not signed in" hint.
pub const KEY_PAGE: &str = "Inventory > Interfaces > CLI";

const AFTER_HELP: &str = "\
Sign in once with `stargazer login` (or set STARGAZER_API_KEY). Create a key in the console: Inventory > Interfaces > CLI.
Output is a table on a terminal and JSON otherwise; force one with --output json|table, trim it with --fields.
Anything that spends credit or destroys something needs --yes. Ordering means you accept https://mhon.cloud/terms.
Exit codes: 0 ok, 1 internal, 2 usage, 3 auth, 4 not found, 5 not enough credit, 6 conflict, 7 rate limited, 8 network, 9 timed out.";

#[derive(Parser, Debug)]
#[command(
    name = "stargazer",
    version,
    about = "Control Stargazer (Mhon Cloud) from the command line: order and manage servers, containers, IPs, DNS names and more.",
    after_help = AFTER_HELP,
    propagate_version = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug)]
pub struct GlobalArgs {
    /// Output format (auto = table on a terminal, JSON otherwise)
    #[arg(long, short = 'o', global = true, value_enum, default_value_t = OutputArg::Auto, env = "STARGAZER_OUTPUT")]
    pub output: OutputArg,

    /// Only these fields in JSON output, comma separated dot paths (e.g. id,name,project.name)
    #[arg(long, global = true, value_name = "FIELDS", value_delimiter = ',')]
    pub fields: Vec<String>,

    /// API base URL [default: https://app.mhon.cloud]
    #[arg(long, global = true, env = "STARGAZER_API_URL", value_name = "URL")]
    pub api_url: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Show the CLI's version
    Version,
    /// Save an API key (read from stdin, or asked for on a terminal)
    Login(LoginArgs),
    /// Remove the saved API key
    Logout,
    /// Show the account the API key belongs to
    Whoami,
    /// Show your credit
    Credit,
    /// Your bills and invoices
    #[command(subcommand)]
    Bills(BillsCmd),
    /// Announcements and messages for you
    #[command(subcommand)]
    Inbox(InboxCmd),
    /// CPU / RAM / disk / network of your servers or containers
    Usage(UsageArgs),
    /// The platform's limits (free containers, DNS names, VPCs per project...)
    Quotas,
    /// Current prices and plans (no key needed)
    Pricing,
    /// Projects: list, show, create, rename, delete
    #[command(subcommand)]
    Projects(ProjectsCmd),
    /// Locations you can order in
    #[command(subcommand)]
    Zones(ZonesCmd),
    /// Virtual machines: order, list, start, stop, resize, renew, delete...
    #[command(subcommand)]
    Servers(ServersCmd),
    /// Containers: order, list, start, stop, logs, delete...
    #[command(subcommand)]
    Containers(ContainersCmd),
    /// DNS names (<name>.in.mhon.cloud): create, point, delete
    #[command(subcommand)]
    Dns(DnsCmd),
    /// Public IPs: reserve, bind to a server, release
    #[command(subcommand, name = "public-ips", visible_alias = "ips")]
    PublicIps(PublicIpsCmd),
    /// SSH public keys: add, install on servers, remove
    #[command(subcommand, name = "ssh-keys", visible_alias = "credentials")]
    SshKeys(SshKeysCmd),
    /// Security groups (firewall rules for servers)
    #[command(subcommand, name = "security-groups", visible_alias = "sg")]
    SecurityGroups(SecurityGroupsCmd),
    /// Server backups: list, back up now, restore, delete
    #[command(subcommand)]
    Backups(BackupsCmd),
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish)
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// `--project` on lists and actions: a project id or name.
#[derive(Args, Debug, Clone)]
pub struct ProjectFilter {
    /// Only this project (id or name)
    #[arg(long, value_name = "PROJECT")]
    pub project: Option<String>,
}

/// `--yes`: what spends credit or destroys something asks for it; without it the command
/// stops with a usage error (exit 2) saying what would happen.
#[derive(Args, Debug, Clone, Copy)]
pub struct Confirm {
    /// Confirm (it spends credit or can't be undone)
    #[arg(long)]
    pub yes: bool,
}

/// `--wait`: block until the queued work is done (or failed), for scripts and agents.
#[derive(Args, Debug, Clone, Copy)]
pub struct Wait {
    /// Wait until it's done (or failed) instead of returning once it's queued
    #[arg(long)]
    pub wait: bool,
    /// Give up waiting after this many seconds (exit 9)
    #[arg(long, default_value_t = 900, value_name = "SECONDS")]
    pub timeout: u64,
}

/// `--period`: months paid now, 1, 6 or 12 (what the Market sells).
fn parse_period(text: &str) -> Result<u32, String> {
    match text {
        "1" | "6" | "12" => Ok(text.parse().unwrap_or(1)),
        _ => Err("1, 6 or 12 (months)".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn cli_is_well_formed() {
        super::Cli::command().debug_assert();
    }
}
