mod cli;
mod config;
mod core;
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
    let mut builder = env_logger::Builder::from_env(env);

    if let Ok(log_file) = std::env::var("AGENT_POKE_LOG_FILE") {
        use std::fs::OpenOptions;
        if let Ok(file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
        } else {
            builder.target(env_logger::Target::Pipe(Box::new(std::io::sink())));
        }
    } else {
        builder.target(env_logger::Target::Pipe(Box::new(std::io::sink())));
    }

    builder.init();
}

fn main() {
    let cli = cli::Cli::parse();
    init_logging(cli.verbose);
    if let Err(e) = cli.run() {
        log::error!("{e:#}");
        std::process::exit(1);
    }
}
