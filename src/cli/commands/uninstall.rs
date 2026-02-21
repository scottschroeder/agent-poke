use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct UninstallCommand {
    #[arg(long)]
    claude: bool,

    #[arg(long)]
    codex: bool,
}

impl UninstallCommand {
    pub fn run(self) -> Result<()> {
        crate::install::run_uninstall(crate::install::UninstallArgs {
            claude: self.claude,
            codex: self.codex,
        })
    }
}
