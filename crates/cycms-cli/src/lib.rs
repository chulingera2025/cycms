mod cli;
mod commands;
mod support;
mod templates;

use cycms_core::Result;

pub use cli::Cli;
use cli::Commands;

/// 运行一次 `cycms` CLI 命令。
///
/// # Errors
/// 当底层配置、数据库、迁移、插件或文件系统操作失败时返回错误。
pub async fn run(cli: Cli) -> Result<()> {
    match &cli.command {
        Commands::New(args) => commands::new::run(args),
        Commands::Serve(args) => commands::serve::run(args).await,
        Commands::Migrate(args) => commands::migrate::run(args).await,
        Commands::Seed(args) => commands::seed::run(args).await,
        Commands::Plugin(args) => commands::plugin::run(args).await,
    }
}
