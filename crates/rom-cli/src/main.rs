mod args;
mod client;
mod input;
mod json;
mod output;
mod sse;
use clap::Parser;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct Failure {
    code: u8,
    message: &'static str,
}
impl Failure {
    fn local(message: &'static str) -> Self {
        Self { code: 2, message }
    }
    fn transport(message: &'static str) -> Self {
        Self { code: 4, message }
    }
    fn uncertain() -> Self {
        Self {
            code: 5,
            message: "mutation outcome unresolved; replay only as the same principal with the original idempotency key, revision and input",
        }
    }
}
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> std::process::ExitCode {
    let cli = match args::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            let _ = error.print();
            return std::process::ExitCode::SUCCESS;
        }
        Err(error) => {
            let message = match error.kind() {
                clap::error::ErrorKind::MissingRequiredArgument
                | clap::error::ErrorKind::MissingSubcommand => {
                    "missing required arguments; use rom COMMAND --help"
                }
                clap::error::ErrorKind::ArgumentConflict => {
                    "incompatible arguments; use rom COMMAND --help"
                }
                clap::error::ErrorKind::InvalidValue | clap::error::ErrorKind::ValueValidation => {
                    "invalid option value; see accepted values in rom COMMAND --help"
                }
                clap::error::ErrorKind::UnknownArgument
                | clap::error::ErrorKind::InvalidSubcommand => {
                    "unsupported command or option; use rom --help"
                }
                _ => "invalid command arguments; use rom COMMAND --help",
            };
            eprintln!("{message}");
            return 2.into();
        }
    };
    let submitted = Arc::new(AtomicBool::new(false));
    let signal_state = submitted.clone();
    // stdout/stdin may block on another process. An independent signal task exits
    // without waiting for that pipe or pretending accepted work was cancelled.
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            if signal_state.load(Ordering::SeqCst) {
                // Exit status conveys uncertainty even when stderr is blocked.
                std::process::exit(130);
            }
            std::process::exit(0);
        }
    });
    let result = async {
        let client = client::Client::new(&cli)?;
        let request = input::request(&cli.command)?;
        client.run(request, cli.output, submitted).await
    }
    .await;
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if error.code == 5 && error.message != Failure::uncertain().message {
                eprintln!("{}", Failure::uncertain().message);
            }
            eprintln!("{}", error.message);
            error.code.into()
        }
    }
}
