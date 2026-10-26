//! Network: DNS names and public IPs (the console's Inventory -> Networks and Market -> Network).
//! VPCs (subnets, internet gateways, route tables) will be groups here too: `stargazer vpcs ...`.

use clap::{Args, Subcommand, ValueEnum};

use super::{Confirm, ProjectFilter};

#[derive(Subcommand, Debug)]
pub enum DnsCmd {
    /// List your DNS names
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
    /// Is a name free?
    Check {
        /// The name (one label: <name>.in.mhon.cloud)
        name: String,
    },
    /// Create a DNS name for a container's port (free)
    Create {
        /// The name (one label: <name>.in.mhon.cloud)
        name: String,
        #[command(flatten)]
        target: DnsTarget,
    },
    /// Point a DNS name at another container / port (the name can't change)
    Update {
        /// DNS name (its name or id)
        record: String,
        #[command(flatten)]
        target: DnsTarget,
    },
    /// Delete a DNS name (needs --yes)
    Delete {
        /// DNS name (its name or id)
        record: String,
        #[command(flatten)]
        confirm: Confirm,
    },
}

#[derive(Args, Debug)]
pub struct DnsTarget {
    /// The container (id, number or name)
    #[arg(long)]
    pub container: String,
    /// Its port (default: its HTTP port, else its first port)
    #[arg(long)]
    pub port: Option<u32>,
    /// web = HTTPS for an HTTP port; direct = A (+ SRV) records for any TCP / UDP port (default: from the port)
    #[arg(long, value_enum)]
    pub kind: Option<DnsKind>,
    /// direct + TCP: add a Minecraft SRV record (players type just the name)
    #[arg(long)]
    pub minecraft_srv: bool,
    #[command(flatten)]
    pub project: ProjectFilter,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum DnsKind {
    Web,
    Direct,
}

#[derive(Subcommand, Debug)]
pub enum PublicIpsCmd {
    /// List your public IPs
    #[command(visible_alias = "ls")]
    List(ProjectFilter),
    /// Price of reserving one, and free addresses per location
    Quote,
    /// Reserve a public IP (monthly from your credit; needs --yes)
    Reserve {
        /// Project id or name (default: your personal Default project)
        #[arg(long)]
        project: Option<String>,
        /// Location id or name (default: the only one with free addresses, if there's one)
        #[arg(long)]
        zone: Option<String>,
        #[command(flatten)]
        confirm: Confirm,
    },
    /// Bind a reserved IP to a server of its project and location (applied after a restart)
    Bind {
        /// The address or its id
        ip: String,
        /// Server id or name
        server: String,
    },
    /// Unbind a reserved IP from its server
    Unbind {
        /// The address or its id
        ip: String,
    },
    /// Release a reserved IP (no refund; needs --yes)
    Release {
        /// The address or its id
        ip: String,
        #[command(flatten)]
        confirm: Confirm,
    },
}
