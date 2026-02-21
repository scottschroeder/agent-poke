use anyhow::Result;

#[derive(Debug, Clone)]
pub struct InstallArgs {
    pub target_dir: Option<String>,
    pub no_claude: bool,
    pub no_codex: bool,
}

#[derive(Debug, Clone)]
pub struct UninstallArgs {
    pub claude: bool,
    pub codex: bool,
}

pub fn run_install(args: InstallArgs) -> Result<()> {
    let target = args
        .target_dir
        .unwrap_or_else(|| "~/.tmux/plugins/agent-poke".to_string());
    println!(
        "Install scaffolding is not implemented yet (target: {target}, claude: {}, codex: {}).",
        !args.no_claude, !args.no_codex
    );
    Ok(())
}

pub fn run_uninstall(args: UninstallArgs) -> Result<()> {
    println!(
        "Uninstall scaffolding is not implemented yet (claude: {}, codex: {}).",
        args.claude, args.codex
    );
    Ok(())
}
