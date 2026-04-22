use std::fs;
use std::path::Path;

use cycms_core::{Error, Result};
use cycms_plugin_manager::{compile_extensions, discover_plugin_dir};

use crate::cli::{
    PluginArgs, PluginCommand, PluginCompileArgs, PluginDisableArgs, PluginEnableArgs,
    PluginInstallArgs, PluginListArgs, PluginNewArgs, PluginRemoveArgs,
};
use crate::commands::new::write_native_plugin_scaffold;
use crate::support::{
    bootstrap_app, canonicalized_eq, copy_dir_recursive, create_dir_all, load_config,
    resolve_plugins_root, write_text_file,
};
use crate::templates::DependencyStyle;

pub(crate) async fn run(args: &PluginArgs) -> Result<()> {
    match &args.command {
        PluginCommand::New(new_args) => run_new(new_args),
        PluginCommand::Compile(compile_args) => run_compile(compile_args),
        PluginCommand::Install(install_args) => run_install(install_args).await,
        PluginCommand::List(list_args) => run_list(list_args).await,
        PluginCommand::Enable(enable_args) => run_enable(enable_args).await,
        PluginCommand::Disable(disable_args) => run_disable(disable_args).await,
        PluginCommand::Remove(remove_args) => run_remove(remove_args).await,
    }
}

fn run_new(args: &PluginNewArgs) -> Result<()> {
    let plugin_name = file_name(&args.name, "plugin")?;
    write_native_plugin_scaffold(&args.name, &plugin_name, &DependencyStyle::Direct)?;
    println!("Created plugin scaffold at {}", args.name.display());
    Ok(())
}

fn run_compile(args: &PluginCompileArgs) -> Result<()> {
    let config = load_config(&args.config)?;
    let plugins_root = resolve_plugins_root(&args.config, &config.plugins.directory);
    let registry = compile_extensions(&plugins_root)?;
    let payload = serde_json::to_string_pretty(&registry).map_err(|source| Error::Internal {
        message: format!("serialize compiled plugin registry: {source}"),
        source: None,
    })?;

    if let Some(output) = &args.output {
        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            create_dir_all(parent)?;
        }
        write_text_file(output, &payload)?;
        println!(
            "Compiled plugin registry written to {} ({} plugins)",
            output.display(),
            registry.plugins.len()
        );
    } else {
        println!("{payload}");
    }

    Ok(())
}

async fn run_install(args: &PluginInstallArgs) -> Result<()> {
    let config = load_config(&args.config)?;
    let source_plugin = discover_plugin_dir(&args.path).map_err(Error::from)?;
    let plugins_root = resolve_plugins_root(&args.config, &config.plugins.directory);
    create_dir_all(&plugins_root)?;
    let target_dir = plugins_root.join(&source_plugin.manifest.plugin.name);

    let staged_dir = if target_dir.exists() {
        if !canonicalized_eq(&args.path, &target_dir)? {
            return Err(Error::Conflict {
                message: format!("plugin directory already exists: {}", target_dir.display()),
            });
        }
        target_dir.clone()
    } else {
        copy_dir_recursive(&args.path, &target_dir)?;
        target_dir.clone()
    };

    let staged_plugin = discover_plugin_dir(&staged_dir).map_err(Error::from)?;
    let copied = !target_dir.exists() || !canonicalized_eq(&args.path, &staged_dir)?;
    let ctx = bootstrap_app(&args.config).await?;
    match ctx.plugin_manager.install(&staged_plugin).await {
        Ok(info) => {
            println!(
                "Installed plugin {} {} from {}",
                info.name,
                info.version,
                staged_dir.display()
            );
            Ok(())
        }
        Err(error) => {
            if copied {
                let _ignore = fs::remove_dir_all(&staged_dir);
            }
            Err(error)
        }
    }
}

async fn run_list(args: &PluginListArgs) -> Result<()> {
    let ctx = bootstrap_app(&args.config).await?;
    let plugins = ctx.plugin_manager.list().await?;
    if plugins.is_empty() {
        println!("No installed plugins.");
        return Ok(());
    }

    for plugin in plugins {
        println!(
            "{}\t{}\t{}\t{}",
            plugin.name,
            plugin.version,
            plugin.kind.as_str(),
            plugin.status.as_str()
        );
    }
    Ok(())
}

async fn run_enable(args: &PluginEnableArgs) -> Result<()> {
    let ctx = bootstrap_app(&args.config).await?;
    ctx.plugin_manager.enable(&args.name).await?;
    println!("Enabled plugin {}", args.name);
    Ok(())
}

async fn run_disable(args: &PluginDisableArgs) -> Result<()> {
    let ctx = bootstrap_app(&args.config).await?;
    ctx.plugin_manager.disable(&args.name, args.force).await?;
    println!("Disabled plugin {}", args.name);
    Ok(())
}

async fn run_remove(args: &PluginRemoveArgs) -> Result<()> {
    let config = load_config(&args.config)?;
    let plugins_root = resolve_plugins_root(&args.config, &config.plugins.directory);
    let plugin_dir = plugins_root.join(&args.name);
    let ctx = bootstrap_app(&args.config).await?;
    ctx.plugin_manager.uninstall(&args.name).await?;
    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir).map_err(|source| Error::Internal {
            message: format!("remove plugin directory {}", plugin_dir.display()),
            source: Some(Box::new(source)),
        })?;
    }
    println!("Removed plugin {}", args.name);
    Ok(())
}

fn file_name(path: &Path, kind: &str) -> Result<String> {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::ValidationError {
            message: format!("{kind} path must end with a directory name"),
            details: None,
        })
}
