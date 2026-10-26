//! Account-wide commands: login, bills, inbox, usage.

use clap::{Args, Subcommand, ValueEnum};

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Read the key from stdin even on a terminal (e.g. `echo "$KEY" | stargazer login --api-key-stdin`)
    #[arg(long)]
    pub api_key_stdin: bool,
}

#[derive(Subcommand, Debug)]
pub enum BillsCmd {
    /// List your bills (newest first)
    #[command(visible_alias = "ls")]
    List {
        /// Only bills issued since (RFC 3339, e.g. 2026-10-01T00:00:00Z)
        #[arg(long)]
        from: Option<String>,
        /// Only bills issued until (RFC 3339)
        #[arg(long)]
        to: Option<String>,
    },
    /// One bill with its items
    Show {
        /// Bill id
        bill: String,
    },
    /// Everything paid in a period, with a total
    Summary {
        #[arg(long, value_enum, default_value_t = Range::D30)]
        range: Range,
    },
    /// Invoices you issued
    Invoices,
    /// One invoice, as issued
    Invoice {
        /// Invoice id
        invoice: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Range {
    #[value(name = "24h")]
    H24,
    #[value(name = "7d")]
    D7,
    #[value(name = "30d")]
    D30,
}

impl Range {
    pub fn as_str(self) -> &'static str {
        match self {
            Range::H24 => "24h",
            Range::D7 => "7d",
            Range::D30 => "30d",
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum InboxCmd {
    /// List announcements and messages (newest first)
    #[command(visible_alias = "ls")]
    List {
        /// Only unread ones
        #[arg(long)]
        unread: bool,
    },
    /// Show one in full and mark it read
    Read {
        /// Announcement id
        id: String,
    },
    /// Remove one from your inbox
    Dismiss {
        /// Announcement id
        id: String,
    },
}

#[derive(Args, Debug)]
pub struct UsageArgs {
    /// Servers or containers
    #[arg(long, value_enum, default_value_t = UsageScope::Servers)]
    pub scope: UsageScope,
    /// One server or container (id or name; default all)
    #[arg(long, value_name = "NAME")]
    pub of: Option<String>,
    /// Period
    #[arg(long, value_enum, default_value_t = UsageRange::D1)]
    pub range: UsageRange,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub enum UsageScope {
    Servers,
    Containers,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum UsageRange {
    #[value(name = "1d")]
    D1,
    #[value(name = "7d")]
    D7,
    #[value(name = "30d")]
    D30,
}

impl UsageRange {
    pub fn as_str(self) -> &'static str {
        match self {
            UsageRange::D1 => "1d",
            UsageRange::D7 => "7d",
            UsageRange::D30 => "30d",
        }
    }
}
