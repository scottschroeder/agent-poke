use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct InstallCommand {
    #[arg(long)]
    target_dir: Option<String>,

    #[arg(long)]
    no_claude: bool,

    #[arg(long)]
    no_codex: bool,
}

impl InstallCommand {
    pub fn run(self) -> Result<()> {
        crate::install::run_install(crate::install::InstallArgs {
            target_dir: self.target_dir,
            no_claude: self.no_claude,
            no_codex: self.no_codex,
        })
    }
}
