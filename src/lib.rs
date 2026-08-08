#![doc = include_str!("../README.md")]

pub mod cli;
mod dock;
pub mod error;
mod install;
mod workspace;
mod zed;

use cli::{Cli, Commands};
use error::{AppError, Result};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Create {
            paths,
            name,
            output,
            mode,
            force,
        } => {
            let output = workspace::create_workspace_file(
                name.as_deref(),
                output.as_deref(),
                mode,
                &paths,
                force,
            )?;
            println!("{}", output.display());

            Ok(())
        }
        Commands::Open {
            workspace,
            mode,
            reuse,
            zed_bin,
        } => {
            let workspace = workspace::resolve_workspace_reference(&workspace)?;
            let workspace_file = workspace::read_workspace_file(&workspace)?;
            let mode = workspace_file.open_mode(mode)?;

            let target = match mode {
                cli::Mode::Folders => {
                    let folders = workspace_file.folder_targets(&workspace)?;
                    zed::OpenTarget::Folders(folders)
                }
                cli::Mode::Symlink => {
                    let folders = workspace_file.resolved_dock_folders(&workspace)?;
                    let dock_root = dock::build_dock(&workspace, &folders)?;
                    zed::OpenTarget::Dock(dock_root)
                }
            };

            zed::open_zed(&zed_bin, target, reuse)
        }
        Commands::Add { paths, workspace } => edit_workspace(
            workspace.as_deref(),
            &paths,
            workspace::WorkspaceEditOperation::Add,
        ),
        Commands::Remove { paths, workspace } => edit_workspace(
            workspace.as_deref(),
            &paths,
            workspace::WorkspaceEditOperation::Remove,
        ),
        Commands::Delete { workspace, force } => delete_workspace(&workspace, force),
        Commands::Install {
            command,
            tasks_path,
        } => install::install_default_tasks(command.as_deref(), tasks_path.as_deref()),
        Commands::List => {
            for workspace in workspace::list_registered_workspaces()? {
                println!("{}\t{}", workspace.name, workspace.path.display());
            }

            Ok(())
        }
    }
}

fn edit_workspace(
    reference: Option<&std::path::Path>,
    paths: &[std::path::PathBuf],
    operation: workspace::WorkspaceEditOperation,
) -> Result<()> {
    let workspace_path = resolve_edit_workspace(reference)?;
    let edit = workspace::prepare_workspace_edit(&workspace_path, paths, operation)?;

    if edit.changed().is_empty() {
        print_edit_result(operation, edit.changed(), edit.unchanged(), false);
        return Ok(());
    }

    let dock = dock::validate_existing_dock(edit.path())?;
    let needs_dock_validation = edit.mode()? == cli::Mode::Symlink || dock.is_some();
    let dock_folders = needs_dock_validation
        .then(|| edit.resolved_dock_folders())
        .transpose()?;

    edit.write()?;
    let dock_synchronized = match dock_folders {
        Some(folders) => match dock::sync_existing_dock(edit.path(), &folders) {
            Ok(synchronized) => synchronized,
            Err(error) => {
                let _ = edit.restore();
                return Err(error);
            }
        },
        None => false,
    };
    print_edit_result(
        operation,
        edit.changed(),
        edit.unchanged(),
        dock_synchronized,
    );

    Ok(())
}

fn resolve_edit_workspace(reference: Option<&std::path::Path>) -> Result<std::path::PathBuf> {
    match reference {
        Some(reference) => workspace::resolve_workspace_reference(reference),
        None => dock::infer_workspace_from_current_dock(),
    }
}

fn print_edit_result(
    operation: workspace::WorkspaceEditOperation,
    changed: &[std::path::PathBuf],
    unchanged: &[std::path::PathBuf],
    dock_synchronized: bool,
) {
    let action = match operation {
        workspace::WorkspaceEditOperation::Add => "added",
        workspace::WorkspaceEditOperation::Remove => "removed",
    };
    let unchanged_action = match operation {
        workspace::WorkspaceEditOperation::Add => "already present",
        workspace::WorkspaceEditOperation::Remove => "already absent",
    };

    for path in changed {
        println!("{action}\t{}", path.display());
    }
    for path in unchanged {
        println!("{unchanged_action}\t{}", path.display());
    }
    if dock_synchronized {
        println!("dock synchronized");
    }
}

fn delete_workspace(reference: &std::path::Path, force: bool) -> Result<()> {
    let workspace = workspace::resolve_workspace_reference(reference)?;
    workspace::read_workspace_file(&workspace)?;
    let workspace_path = std::fs::canonicalize(&workspace)?;
    let dock = dock::validate_existing_dock(&workspace_path)?;

    println!("workspace\t{}", workspace_path.display());
    match &dock {
        Some(path) => println!("dock\t{}", path.display()),
        None => println!("dock\tnot materialized"),
    }

    if !force {
        println!("dry-run\tpass --force to delete");
        return Ok(());
    }
    if dock::current_directory_is_dock(&workspace_path)? {
        return Err(AppError::CurrentDirectoryIsDock {
            path: workspace_path,
        });
    }

    if dock.is_some() {
        dock::remove_owned_dock(&workspace_path)?;
    }
    std::fs::remove_file(workspace_path)?;
    println!("deleted");

    Ok(())
}
