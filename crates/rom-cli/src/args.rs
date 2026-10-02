use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "rom",
    version,
    about = "Read and operate registered ROM Resources over HTTP",
    after_help = "Mutations require an explicit idempotency key and, except create, expected revision.\nAn unavailable reply does not prove rollback. Replay the original request only as the same principal.\nNo automatic retries, saved credentials, login, or stream reconnection."
)]
pub struct Cli {
    /// Server base URL (HTTPS or numeric loopback HTTP)
    #[arg(long, global = true)]
    pub endpoint: Option<String>,
    /// File containing the complete Authorization header value; never an inline credential
    #[arg(long, global = true)]
    pub auth_file: Option<std::path::PathBuf>,
    #[arg(long, global = true, value_enum, default_value = "human")]
    pub output: Format,
    /// Connect timeout in seconds (1..=300)
    #[arg(long, global = true, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=300))]
    pub connect_timeout: u64,
    /// Finite request/header timeout in seconds (1..=3600)
    #[arg(long, global = true, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub request_timeout: u64,
    /// Stream inactivity timeout in seconds, including keepalive activity (1..=3600)
    #[arg(long, global = true, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub idle_timeout: u64,
    #[command(subcommand)]
    pub command: Command,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Human,
    Json,
}
#[derive(Subcommand)]
pub enum Command {
    /// Inspect explicitly disclosed metadata; it grants no operation permission
    Discover {
        kind: Option<String>,
    },
    Read {
        kind: String,
        id: String,
    },
    /// Bounded equality query; defaults to all, in moving ID order
    Query(Query),
    Create {
        kind: String,
        id: String,
        #[arg(long)]
        idempotency: String,
        #[arg(long)]
        input_file: String,
    },
    Replace {
        #[command(flatten)]
        mutation: Mutation,
        #[arg(long)]
        input_file: String,
    },
    /// Explicit field updates: {"field":{"op":"set","value":...}} or {"op":"remove"}
    Patch {
        #[command(flatten)]
        mutation: Mutation,
        #[arg(long)]
        input_file: String,
    },
    Delete(Mutation),
    Action {
        #[command(flatten)]
        mutation: Mutation,
        name: String,
        #[arg(long)]
        input_file: String,
    },
    /// Submit an exact Invocation envelope, including its original key and expected revision
    Invoke {
        #[arg(long)]
        request_file: String,
    },
    /// Observe whole authorized snapshots; stops on stream loss, no resume cursor
    Live(Query),
    Journal(Journal),
    /// Establish a new cursor explicitly; this does not recover lost history
    JournalHead {
        kind: String,
    },
    /// Observe journal batches, preserving each batch and cursor together
    Subscribe(Journal),
}
#[derive(Args)]
pub struct Query {
    pub kind: String,
    #[arg(long)]
    pub query_file: Option<String>,
}
#[derive(Args)]
pub struct Journal {
    pub kind: String,
    #[arg(long)]
    pub after_file: Option<String>,
}
#[derive(Args)]
pub struct Mutation {
    pub kind: String,
    pub id: String,
    #[arg(long)]
    pub expected: u64,
    #[arg(long)]
    pub idempotency: String,
}
