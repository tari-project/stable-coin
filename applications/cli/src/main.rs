use crate::cli::{Cli, Command};
use crate::context::CliContext;

mod cli;
mod context;
mod print_result;
mod transactions;
mod value_parsers;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::init();

    let context = CliContext::new(cli.common);
    match cli.command {
        Command::Issuer(cmd) => cmd.run(context).await?,
    }

    Ok(())
}
