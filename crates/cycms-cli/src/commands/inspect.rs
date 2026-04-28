use cycms_core::Result;
use cycms_host_types::HostRequestTarget;
use cycms_plugin_manager::{HostRegistry, RegistryLookup, compile_extensions};

use crate::cli::{InspectArgs, InspectCommand};
use crate::support::{load_config, resolve_plugins_root};

pub(crate) fn run(args: &InspectArgs) -> Result<()> {
    match &args.command {
        InspectCommand::Registry => run_registry(),
        InspectCommand::Route { path } => run_route(path),
    }
}

fn run_registry() -> Result<()> {
    let config = load_config(&std::path::PathBuf::from("cycms.toml"))?;
    let plugins_root = resolve_plugins_root(&std::path::PathBuf::from("cycms.toml"), &config.plugins.directory);
    let compiled = compile_extensions(&plugins_root)?;
    let registry = HostRegistry::new(compiled);
    let snapshot = registry.diagnostics_snapshot();
    let payload = serde_json::to_string_pretty(&snapshot).map_err(|source| cycms_core::Error::Internal {
        message: format!("serialize diagnostics snapshot: {source}"),
        source: None,
    })?;
    println!("{payload}");
    Ok(())
}

fn run_route(path: &str) -> Result<()> {
    let config = load_config(&std::path::PathBuf::from("cycms.toml"))?;
    let plugins_root = resolve_plugins_root(&std::path::PathBuf::from("cycms.toml"), &config.plugins.directory);
    let compiled = compile_extensions(&plugins_root)?;
    let registry = HostRegistry::new(compiled);

    let request = HostRequestTarget { path: path.to_owned() };
    let public_decision = registry.resolve_public_page(&request);
    let admin_decision = registry.resolve_admin_page(&request);

    let public_has_candidates = !public_decision.diagnostics.candidates.is_empty();
    let admin_has_candidates = !admin_decision.diagnostics.candidates.is_empty();

    if !public_has_candidates && !admin_has_candidates {
        println!("No matching route found for path: {path}");
        return Ok(());
    }

    let output = serde_json::json!({
        "path": path,
        "public": public_decision,
        "admin": admin_decision,
    });
    let payload = serde_json::to_string_pretty(&output).map_err(|source| cycms_core::Error::Internal {
        message: format!("serialize route resolution: {source}"),
        source: None,
    })?;
    println!("{payload}");
    Ok(())
}
