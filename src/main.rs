mod cli;
mod config;
mod core;
mod install;
mod notify;
mod tmux;

use clap::Parser;

fn init_logging(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };

    let env = env_logger::Env::default().default_filter_or(level);
    env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .init();
}

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    init_logging(cli.verbose);
    cli.run()
}
