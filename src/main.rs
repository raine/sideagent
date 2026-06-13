use anyhow::Result;
use clap::Parser;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use std::path::PathBuf;

mod config;
mod git_worktree;
mod headless;
mod install_skill;
mod launcher;
mod monitor;
mod prompt;
mod run;
mod run_dir;
mod tmux;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(name = "sideagent")]
#[command(version)]
#[command(about = "Run another coding agent from your current session")]
#[command(styles = STYLES)]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    run: RunArgs,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Launch a profile with a prompt and wait for completion.
    Run(RunArgs),

    /// Monitor headless run archives in a terminal UI.
    Monitor(MonitorArgs),

    /// List configured profiles.
    Profiles(ConfigArgs),

    /// Install the bundled skill for supported providers.
    InstallSkill(InstallSkillArgs),
}

#[derive(clap::Args)]
struct InstallSkillArgs {
    /// Install only this provider.
    #[arg(long, value_enum)]
    provider: Option<install_skill::Provider>,
}

#[derive(clap::Args)]
struct RunArgs {
    /// Profile name from the selected config.
    #[arg(short, long)]
    profile: Option<String>,

    /// Use this config file instead of config discovery.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Run headlessly without tmux.
    #[arg(short = 'H', long)]
    headless: bool,

    /// Prompt text. If omitted, the prompt is read from stdin.
    #[arg(trailing_var_arg = true)]
    prompt: Vec<String>,
}

#[derive(clap::Args)]
struct ConfigArgs {
    /// Use this config file instead of config discovery.
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(clap::Args, Debug, Clone)]
pub(crate) struct MonitorArgs {
    /// Read run archives from this directory instead of the default state directory.
    #[arg(long)]
    runs_root: Option<PathBuf>,

    /// Poll interval for refreshing runs and transcript output.
    #[arg(long, default_value_t = 500)]
    poll_interval_ms: u64,

    /// Render one snapshot to stdout and exit without entering the TUI.
    #[arg(long, hide = true)]
    once: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run(args)) => run::run(args),
        Some(Commands::Monitor(args)) => monitor::run(args),
        Some(Commands::Profiles(args)) => run::profiles(args),
        Some(Commands::InstallSkill(args)) => install_skill::run(args.provider),
        None => run::run(cli.run),
    }
}
