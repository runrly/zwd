use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use tempfile::tempdir;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_zwd")
}

fn run(args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(binary()).args(args).output()?)
}

fn run_with_home_and_cwd(args: &[&str], home: &Path, cwd: &Path) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(binary())
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .current_dir(cwd)
        .output()?)
}

fn stdout_line(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn open_dock(workspace: &Path, home: &Path, cwd: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let output = run_with_home_and_cwd(
        &["open", &workspace_arg, "--reuse", "--zed-bin", "/bin/echo"],
        home,
        cwd,
    )?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(PathBuf::from(stdout_line(&output)))
}

fn folder_paths(workspace: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let document: Value = serde_json::from_str(&fs::read_to_string(workspace)?)?;
    let folders = document["folders"]
        .as_array()
        .ok_or("workspace folders must be an array")?;

    folders
        .iter()
        .map(|folder| {
            folder["path"]
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "workspace folder path must be a string".into())
        })
        .collect()
}

#[test]
fn public_command_launches_cli() -> Result<(), Box<dyn Error>> {
    let output = Command::new(binary()).arg("--help").output()?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Usage: zwd <COMMAND>"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    Ok(())
}

#[test]
fn create_writes_code_workspace_with_dock_config() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project_root = temp.path().join("project");
    let api = project_root.join("api");
    let web = project_root.join("web");
    let output_dir = temp.path().join("workspaces");
    fs::create_dir_all(&api)?;
    fs::create_dir_all(&web)?;
    let output_dir_arg = output_dir.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &[
            "create",
            "api",
            "web",
            "--name",
            "demo",
            "--output",
            &output_dir_arg,
        ],
        temp.path(),
        &project_root,
    )?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let workspace = output_dir.join("demo.code-workspace");
    let document: Value = serde_json::from_str(&fs::read_to_string(&workspace)?)?;

    assert_eq!(document["zwd"]["mode"], "symlink");
    assert!(document["folders"][0].get("name").is_none());
    assert_eq!(
        document["folders"][0]["path"],
        api.canonicalize()?.to_string_lossy().into_owned()
    );
    assert_eq!(
        document["folders"][1]["path"],
        web.canonicalize()?.to_string_lossy().into_owned()
    );
    assert_eq!(
        stdout_line(&output),
        workspace.canonicalize()?.to_string_lossy().into_owned()
    );

    Ok(())
}

#[test]
fn create_without_name_registers_generated_workspace_and_prints_path() -> Result<(), Box<dyn Error>>
{
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("project");
    let api = project_root.join("api");
    fs::create_dir_all(&api)?;

    let output = run_with_home_and_cwd(&["create", "api"], &home, &project_root)?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let workspace = stdout_line(&output);
    let workspace_path = Path::new(&workspace);
    let file_name = workspace_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("workspace path must have a UTF-8 file name");
    let document: Value = serde_json::from_str(&fs::read_to_string(workspace_path)?)?;

    assert!(file_name.starts_with("ws-"));
    assert!(file_name.ends_with(".code-workspace"));
    assert_eq!(document["zwd"]["mode"], "symlink");
    assert_eq!(
        document["folders"][0]["path"],
        api.canonicalize()?.to_string_lossy().into_owned()
    );

    Ok(())
}

#[test]
fn create_output_without_name_generates_workspace_in_output_dir() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project_root = temp.path().join("project");
    let api = project_root.join("api");
    let output_dir = temp.path().join("workspaces");
    fs::create_dir_all(&api)?;
    let output_dir_arg = output_dir.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["create", "api", "--output", &output_dir_arg],
        temp.path(),
        &project_root,
    )?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let workspace = stdout_line(&output);
    let workspace_path = Path::new(&workspace);
    let file_name = workspace_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("workspace path must have a UTF-8 file name");

    assert!(file_name.starts_with("ws-"));
    assert!(workspace_path.starts_with(output_dir.canonicalize()?));

    Ok(())
}

#[test]
fn create_list_and_open_registered_workspace_by_name() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("project");
    let api = project_root.join("api");
    fs::create_dir_all(&api)?;

    let create = run_with_home_and_cwd(
        &["create", "api", "--name", "custom", "--mode", "folders"],
        &home,
        &project_root,
    )?;

    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );

    let workspace = stdout_line(&create);
    fs::create_dir(project_root.join("custom"))?;
    let list = run_with_home_and_cwd(&["list"], &home, &project_root)?;
    let open = run_with_home_and_cwd(
        &["open", "custom", "--reuse", "--zed-bin", "/bin/echo"],
        &home,
        &project_root,
    )?;
    let open_with_extension = run_with_home_and_cwd(
        &[
            "open",
            "custom.code-workspace",
            "--reuse",
            "--zed-bin",
            "/bin/echo",
        ],
        &home,
        &project_root,
    )?;

    assert_eq!(
        String::from_utf8_lossy(&list.stdout).trim(),
        format!("custom\t{workspace}")
    );
    assert!(
        open.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&open.stderr)
    );
    assert_eq!(stdout_line(&open), api.canonicalize()?.to_string_lossy());
    assert!(
        open_with_extension.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&open_with_extension.stderr)
    );
    assert_eq!(
        stdout_line(&open_with_extension),
        api.canonicalize()?.to_string_lossy()
    );

    Ok(())
}

#[test]
fn create_requires_force_to_overwrite_output_workspace() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project_root = temp.path().join("project");
    let api = project_root.join("api");
    let output_dir = temp.path().join("workspaces");
    fs::create_dir_all(&api)?;
    fs::create_dir_all(&output_dir)?;
    fs::write(output_dir.join("demo.code-workspace"), "{}")?;
    let output_dir_arg = output_dir.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &[
            "create",
            "api",
            "--name",
            "demo",
            "--output",
            &output_dir_arg,
        ],
        temp.path(),
        &project_root,
    )?;
    let forced = run_with_home_and_cwd(
        &[
            "create",
            "api",
            "--name",
            "demo",
            "--output",
            &output_dir_arg,
            "--force",
        ],
        temp.path(),
        &project_root,
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--force"));
    assert!(
        forced.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&forced.stderr)
    );

    Ok(())
}

#[test]
fn open_folders_mode_invokes_zed_binary_with_resolved_folder() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [{ "name": "api", "path": "api" }],
          "zwd": { "mode": "folders" }
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&[
        "open",
        &workspace_arg,
        "--mode",
        "folders",
        "--reuse",
        "--zed-bin",
        "/bin/echo",
    ])?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        project.canonicalize()?.to_string_lossy()
    );

    Ok(())
}

#[test]
fn open_rejects_non_code_workspace_input() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let workspace = temp.path().join("demo.zed-workspace");
    fs::write(&workspace, "{}")?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&["open", &workspace_arg, "--reuse", "--zed-bin", "/bin/echo"])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(".code-workspace"));

    Ok(())
}

#[test]
fn open_rejects_legacy_zed_dock_workspace_key() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [{ "name": "api", "path": "api" }],
          "zed-dock": { "mode": "symlink" }
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&["open", &workspace_arg, "--reuse", "--zed-bin", "/bin/echo"])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(r#"rename it to "zwd""#));

    Ok(())
}

#[test]
fn open_rejects_null_legacy_zed_dock_workspace_key() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [{ "name": "api", "path": "api" }],
          "zed-dock": null
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&["open", &workspace_arg, "--reuse", "--zed-bin", "/bin/echo"])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(r#"rename it to "zwd""#));

    Ok(())
}

#[test]
fn create_rejects_output_file_path() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    let project_arg = project.to_string_lossy().into_owned();
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&["create", &project_arg, "--output", &workspace_arg])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--output expects a directory"));
    assert!(!workspace.exists());

    Ok(())
}

#[test]
fn create_rejects_removed_folder_flag() -> Result<(), Box<dyn Error>> {
    let output = run(&["create", "api", "--folder", "api"])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument '--folder'"));

    Ok(())
}

#[test]
fn create_rejects_missing_folder_path() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let missing = temp.path().join("missing");
    let missing_arg = missing.to_string_lossy().into_owned();

    let output = run(&["create", &missing_arg])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("folder path does not exist"));

    Ok(())
}

#[test]
fn create_rejects_non_directory_path() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let file = temp.path().join("file.txt");
    fs::write(&file, "")?;
    let file_arg = file.to_string_lossy().into_owned();

    let output = run(&["create", &file_arg])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("folder target is not a directory"));

    Ok(())
}

#[test]
fn create_symlink_mode_rejects_duplicate_basenames() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let left = temp.path().join("left/api");
    let right = temp.path().join("right/api");
    fs::create_dir_all(&left)?;
    fs::create_dir_all(&right)?;
    let left_arg = left.to_string_lossy().into_owned();
    let right_arg = right.to_string_lossy().into_owned();

    let output = run(&["create", &left_arg, &right_arg])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate folder name"));

    Ok(())
}

#[test]
fn create_folders_mode_allows_duplicate_basenames() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let left = temp.path().join("left/api");
    let right = temp.path().join("right/api");
    let output_dir = temp.path().join("workspaces");
    fs::create_dir_all(&left)?;
    fs::create_dir_all(&right)?;
    let left_arg = left.to_string_lossy().into_owned();
    let right_arg = right.to_string_lossy().into_owned();
    let output_dir_arg = output_dir.to_string_lossy().into_owned();

    let output = run(&[
        "create",
        &left_arg,
        &right_arg,
        "--mode",
        "folders",
        "--name",
        "demo",
        "--output",
        &output_dir_arg,
    ])?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[test]
fn open_rejects_parent_dir_link_name_before_launching_zed() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [{ "name": "../x", "path": "api" }],
          "zwd": { "mode": "symlink" }
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&[
        "open",
        &workspace_arg,
        "--mode",
        "symlink",
        "--reuse",
        "--zed-bin",
        "/bin/false",
    ])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid dock link name"));

    Ok(())
}

#[test]
fn open_folders_mode_allows_invalid_link_name() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [{ "name": "../x", "path": "api" }],
          "zwd": { "mode": "folders" }
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&["open", &workspace_arg, "--reuse", "--zed-bin", "/bin/echo"])?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        project.canonicalize()?.to_string_lossy()
    );

    Ok(())
}

#[test]
fn install_writes_root_array_from_resource() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let tasks_path = temp.path().join("tasks.json");
    let tasks_path_arg = tasks_path.to_string_lossy().into_owned();
    let command_path = temp.path().join("zwd");
    fs::write(&command_path, "")?;
    let command_arg = command_path.to_string_lossy().into_owned();

    let output = run(&[
        "install",
        "--tasks-path",
        &tasks_path_arg,
        "--command",
        &command_arg,
    ])?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_str(&fs::read_to_string(tasks_path)?)?;
    let tasks = document.as_array().expect("tasks json must be an array");
    let expected_command = command_path.canonicalize()?.to_string_lossy().into_owned();

    assert_eq!(tasks.len(), 3);
    assert!(tasks.iter().all(|task| task["command"] == expected_command));
    assert!(tasks.iter().all(|task| task["args"][1] == "$ZED_FILE"));

    Ok(())
}

#[test]
fn open_symlink_mode_rejects_lock_link_name_before_launching_zed() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [{ "name": ".zwd-lock.json", "path": "api" }],
          "zwd": { "mode": "symlink" }
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&[
        "open",
        &workspace_arg,
        "--mode",
        "symlink",
        "--reuse",
        "--zed-bin",
        "/bin/false",
    ])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("reserved dock metadata name"));

    Ok(())
}

#[test]
fn open_symlink_rejects_case_insensitive_duplicate_links_before_zed() -> Result<(), Box<dyn Error>>
{
    let temp = tempdir()?;
    let api = temp.path().join("api");
    let web = temp.path().join("web");
    fs::create_dir(&api)?;
    fs::create_dir(&web)?;
    let workspace = temp.path().join("demo.code-workspace");
    fs::write(
        &workspace,
        r#"{
          "folders": [
            { "name": "api", "path": "api" },
            { "name": "API", "path": "web" }
          ],
          "zwd": { "mode": "symlink" }
        }"#,
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run(&[
        "open",
        &workspace_arg,
        "--mode",
        "symlink",
        "--reuse",
        "--zed-bin",
        "/bin/false",
    ])?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("duplicate folder name"));

    Ok(())
}

#[test]
fn add_updates_existing_dock_and_preserves_workspace_properties() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let web = project_root.join("web");
    let docs = project_root.join("docs");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::create_dir_all(&web)?;
    fs::create_dir_all(&docs)?;
    fs::write(
        &workspace,
        format!(
            r#"{{
              "folders": [{{ "name": "api-root", "path": {:?} }}],
              "settings": {{ "theme": "dark" }},
              "zwd": {{ "mode": "symlink" }}
            }}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let web_arg = web.to_string_lossy().into_owned();
    let docs_arg = docs.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["add", &web_arg, &docs_arg, "--workspace", &workspace_arg],
        &home,
        &project_root,
    )?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        folder_paths(&workspace)?,
        vec![
            api.canonicalize()?.to_string_lossy().into_owned(),
            web.canonicalize()?.to_string_lossy().into_owned(),
            docs.canonicalize()?.to_string_lossy().into_owned(),
        ]
    );
    let document: Value = serde_json::from_str(&fs::read_to_string(&workspace)?)?;
    assert_eq!(document["settings"]["theme"], "dark");
    assert_eq!(document["folders"][0]["name"], "api-root");
    assert!(
        dock.join("web")
            .symlink_metadata()?
            .file_type()
            .is_symlink()
    );
    assert!(
        dock.join("docs")
            .symlink_metadata()?
            .file_type()
            .is_symlink()
    );

    Ok(())
}

#[test]
fn add_infers_workspace_from_dock_root() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let docs = project_root.join("docs");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::create_dir_all(&docs)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"symlink"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let docs_arg = docs.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(&["add", &docs_arg], &home, &dock)?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(folder_paths(&workspace)?.len(), 2);
    assert!(
        dock.join("docs")
            .symlink_metadata()?
            .file_type()
            .is_symlink()
    );

    Ok(())
}

#[test]
fn remove_updates_dock_without_deleting_project_directories() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let docs = project_root.join("docs");
    let missing = project_root.join("missing");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::create_dir_all(&docs)?;
    fs::create_dir_all(&missing)?;
    fs::write(
        &workspace,
        format!(
            r#"{{
              "folders": [{{"path":{:?}}}, {{"path":{:?}}}],
              "zwd": {{"mode":"symlink"}}
            }}"#,
            api.canonicalize()?,
            docs.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let docs_arg = docs.to_string_lossy().into_owned();
    let missing_arg = missing.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &[
            "remove",
            &docs_arg,
            &missing_arg,
            "--workspace",
            &workspace_arg,
        ],
        &home,
        &project_root,
    )?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(folder_paths(&workspace)?.len(), 1);
    assert!(!dock.join("docs").exists());
    assert!(docs.is_dir());

    Ok(())
}

#[test]
fn add_aborts_without_changing_workspace_when_dock_has_unmanaged_content()
-> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let docs = project_root.join("docs");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::create_dir_all(&docs)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"symlink"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    fs::write(dock.join("notes.txt"), "do not touch")?;
    let before = fs::read_to_string(&workspace)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let docs_arg = docs.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["add", &docs_arg, "--workspace", &workspace_arg],
        &home,
        &project_root,
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unmanaged content"));
    assert_eq!(fs::read_to_string(&workspace)?, before);

    Ok(())
}

#[test]
fn remove_allows_an_empty_workspace() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"folders"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let api_arg = api.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["remove", &api_arg, "--workspace", &workspace_arg],
        &home,
        &project_root,
    )?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(folder_paths(&workspace)?.is_empty());

    Ok(())
}

#[test]
fn delete_previews_then_removes_owned_dock_and_workspace() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"symlink"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let preview = run_with_home_and_cwd(&["delete", &workspace_arg], &home, &project_root)?;

    assert!(
        preview.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert!(workspace.exists());
    assert!(dock.exists());

    let deleted =
        run_with_home_and_cwd(&["delete", &workspace_arg, "--force"], &home, &project_root)?;

    assert!(
        deleted.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&deleted.stderr)
    );
    assert!(!workspace.exists());
    assert!(!dock.exists());
    assert!(api.is_dir());

    Ok(())
}

#[test]
#[ignore = "enabled with the status command"]
fn status_reports_healthy_and_unhealthy_dock_states() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"symlink"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let healthy = run_with_home_and_cwd(&["status", &workspace_arg], &home, &project_root)?;

    assert!(
        healthy.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&healthy.stderr)
    );
    assert!(stdout_line(&healthy).contains("healthy"));

    fs::write(dock.join("notes.txt"), "unmanaged")?;
    let unhealthy = run_with_home_and_cwd(&["status", &workspace_arg], &home, &project_root)?;

    assert!(!unhealthy.status.success());
    assert!(stdout_line(&unhealthy).contains("unhealthy"));

    Ok(())
}

#[test]
fn add_reports_existing_folder_without_rewriting_workspace_or_dock() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"symlink"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let before_workspace = fs::read_to_string(&workspace)?;
    let before_lock = fs::read_to_string(dock.join(".zwd-lock.json"))?;
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let api_arg = api.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["add", &api_arg, "--workspace", &workspace_arg],
        &home,
        &project_root,
    )?;

    assert!(output.status.success());
    assert!(stdout_line(&output).contains("already present"));
    assert_eq!(fs::read_to_string(&workspace)?, before_workspace);
    assert_eq!(
        fs::read_to_string(dock.join(".zwd-lock.json"))?,
        before_lock
    );

    Ok(())
}

#[test]
fn remove_deletes_all_legacy_entries_for_the_same_folder() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{
              "folders": [{{"path":{:?}}}, {{"path":{:?}}}],
              "zwd": {{"mode":"folders"}}
            }}"#,
            api.canonicalize()?,
            api.canonicalize()?
        ),
    )?;
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let api_arg = api.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["remove", &api_arg, "--workspace", &workspace_arg],
        &home,
        &project_root,
    )?;

    assert!(output.status.success());
    assert!(folder_paths(&workspace)?.is_empty());

    Ok(())
}

#[test]
fn delete_refuses_to_remove_the_current_dock_root() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"symlink"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    let dock = open_dock(&workspace, &home, &project_root)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(&["delete", &workspace_arg, "--force"], &home, &dock)?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("leave"));
    assert!(workspace.exists());
    assert!(dock.exists());

    Ok(())
}

#[test]
#[ignore = "enabled with the status command"]
fn status_lists_missing_workspace_folders_before_returning_an_error() -> Result<(), Box<dyn Error>>
{
    let temp = tempdir()?;
    let home = temp.path().join("home");
    let project_root = temp.path().join("projects");
    let api = project_root.join("api");
    let workspace = temp.path().join("demo.code-workspace");
    fs::create_dir_all(&api)?;
    fs::write(
        &workspace,
        format!(
            r#"{{"folders":[{{"path":{:?}}}],"zwd":{{"mode":"folders"}}}}"#,
            api.canonicalize()?
        ),
    )?;
    fs::remove_dir(&api)?;
    let workspace_arg = workspace.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(&["status", &workspace_arg], &home, &project_root)?;

    assert!(!output.status.success());
    assert!(stdout_line(&output).contains("folder\tunhealthy"));

    Ok(())
}
