use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkCommand {
    /// Inspect currently disclosed operator capabilities
    Capabilities,
    /// Inspect one bounded page; the query defaults to limit 64
    List {
        #[arg(long)]
        query_file: Option<String>,
    },
    /// Inspect a work handle from an authorized view
    Show { handle: String },
    /// Schedule unchanged work using an exact saved request envelope
    Retry {
        #[arg(long)]
        request_file: String,
    },
    /// Reconcile unchanged work using an exact saved request envelope
    Reconcile {
        #[arg(long)]
        request_file: String,
    },
}
