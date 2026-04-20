use std::path::Path;

use cycms_core::{Error, Result};

use crate::cli::NewArgs;
use crate::support::{create_dir_all, ensure_absent, write_text_file};
use crate::templates::{
    DEFAULT_CONFIG_TEMPLATE, DependencyStyle, render_native_plugin_scaffold,
    render_project_workspace,
};

pub(crate) fn run(args: &NewArgs) -> Result<()> {
    let project_root = &args.project_name;
    ensure_absent(project_root, "project directory")?;

    create_dir_all(project_root)?;
    create_dir_all(&project_root.join("migrations/postgres"))?;
    create_dir_all(&project_root.join("migrations/mysql"))?;
    create_dir_all(&project_root.join("migrations/sqlite"))?;
    create_dir_all(&project_root.join("plugins"))?;

    let example_plugin_name = "example-plugin";
    write_text_file(
        &project_root.join("Cargo.toml"),
        &render_project_workspace(example_plugin_name),
    )?;
    write_text_file(&project_root.join("cycms.toml"), DEFAULT_CONFIG_TEMPLATE)?;

    write_native_plugin_scaffold(
        &project_root.join("plugins").join(example_plugin_name),
        example_plugin_name,
        &DependencyStyle::Workspace,
    )?;

    println!(
        "Created cycms project skeleton at {}",
        project_root.display()
    );
    Ok(())
}

pub(crate) fn write_native_plugin_scaffold(
    root: &Path,
    plugin_name: &str,
    dependency_style: &DependencyStyle,
) -> Result<()> {
    if plugin_name.trim().is_empty() {
        return Err(Error::ValidationError {
            message: "plugin name must not be empty".to_owned(),
            details: None,
        });
    }

    ensure_absent(root, "plugin directory")?;
    let rendered = render_native_plugin_scaffold(plugin_name, dependency_style)?;

    create_dir_all(root)?;
    create_dir_all(&root.join("src"))?;
    create_dir_all(&root.join("migrations/postgres"))?;
    create_dir_all(&root.join("migrations/mysql"))?;
    create_dir_all(&root.join("migrations/sqlite"))?;

    write_text_file(&root.join("Cargo.toml"), &rendered.cargo_toml)?;
    write_text_file(&root.join("plugin.toml"), &rendered.plugin_toml)?;
    write_text_file(&root.join("src/lib.rs"), &rendered.lib_rs)?;
    Ok(())
}
