use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cycms", version, about = "cycms command line interface", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    New(NewArgs),
    Serve(ServeArgs),
    Migrate(MigrateArgs),
    Seed(SeedArgs),
    Plugin(PluginArgs),
    Inspect(InspectArgs),
}

#[derive(Debug, Args)]
pub struct NewArgs {
    pub project_name: PathBuf,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Args)]
pub struct MigrateArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    #[command(subcommand)]
    pub command: MigrateCommand,
}

#[derive(Debug, Subcommand)]
pub enum MigrateCommand {
    Run,
    Rollback {
        #[arg(long, default_value_t = 1)]
        count: usize,
    },
    Status,
}

#[derive(Debug, Args)]
pub struct SeedArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    #[arg(long, default_value = "admin")]
    pub username: String,
    #[arg(long, default_value = "admin@example.local")]
    pub email: String,
    #[arg(long)]
    pub password: Option<String>,
}

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    New(PluginNewArgs),
    Compile(PluginCompileArgs),
    Install(PluginInstallArgs),
    List(PluginListArgs),
    Enable(PluginEnableArgs),
    Disable(PluginDisableArgs),
    Remove(PluginRemoveArgs),
}

#[derive(Debug, Args)]
pub struct PluginNewArgs {
    pub name: PathBuf,
}

#[derive(Debug, Args)]
pub struct PluginInstallArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct PluginCompileArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct PluginListArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
}

#[derive(Debug, Args)]
pub struct PluginEnableArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    pub name: String,
}

#[derive(Debug, Args)]
pub struct PluginDisableArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    pub name: String,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct PluginRemoveArgs {
    #[arg(long, default_value = "cycms.toml")]
    pub config: PathBuf,
    pub name: String,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[command(subcommand)]
    pub command: InspectCommand,
}

#[derive(Debug, Subcommand)]
pub enum InspectCommand {
    Registry,
    Route {
        path: String,
    },
}
