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
        .env("XDG_CACHE_HOME", home.join(".cache"))
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
    fs::write(dock.join("notes.txt"), "keep")?;
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
    assert_eq!(fs::read_to_string(dock.join("notes.txt"))?, "keep");

    Ok(())
}

#[test]
fn add_preserves_unmanaged_content() -> Result<(), Box<dyn Error>> {
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
    let workspace_arg = workspace.to_string_lossy().into_owned();
    let docs_arg = docs.to_string_lossy().into_owned();

    let output = run_with_home_and_cwd(
        &["add", &docs_arg, "--workspace", &workspace_arg],
        &home,
        &project_root,
    )?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(dock.join("notes.txt"))?, "do not touch");
    assert!(dock.join("docs").is_dir());

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
    let extras = run_with_home_and_cwd(&["status", &workspace_arg], &home, &project_root)?;
    assert!(extras.status.success());
    fs::remove_file(dock.join("api"))?;
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

#[test]
fn sync_creates_recovers_and_reconciles_without_opening_zed() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let api = temp.path().join("api");
    let web = temp.path().join("web");
    fs::create_dir(&api)?;
    fs::create_dir(&web)?;
    let workspace = temp.path().join("work.code-workspace");
    let document = serde_json::json!({"folders":[{"path":api}], "zwd":{"mode":"symlink"}});
    fs::write(&workspace, document.to_string())?;
    let arg = workspace.to_string_lossy();
    let sync = run_with_home_and_cwd(&["sync", &arg], temp.path(), temp.path())?;
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );
    let dock = open_dock(&workspace, temp.path(), temp.path())?;
    fs::create_dir(dock.join(".dirac"))?;
    fs::write(dock.join(".dirac/config.json"), "{}")?;
    fs::write(dock.join("notes.txt"), "keep")?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&api, dock.join("manual-link"))?;
    fs::remove_file(dock.join("api"))?;
    let repaired = run_with_home_and_cwd(&["sync"], temp.path(), &dock)?;
    assert!(
        repaired.status.success(),
        "{}",
        String::from_utf8_lossy(&repaired.stderr)
    );
    assert!(dock.join("api").is_dir());
    let before = fs::read(&workspace)?;
    let lock = fs::read(dock.join(".zwd-lock.json"))?;
    let again = run_with_home_and_cwd(&["sync", &arg], temp.path(), temp.path())?;
    assert!(again.status.success());
    assert_eq!(fs::read(&workspace)?, before);
    assert_eq!(fs::read(dock.join(".zwd-lock.json"))?, lock);
    fs::write(
        &workspace,
        serde_json::json!({"folders":[{"path":web}], "zwd":{"mode":"symlink"}}).to_string(),
    )?;
    open_dock(&workspace, temp.path(), temp.path())?;
    assert!(!dock.join("api").exists());
    assert!(dock.join("web").is_dir());
    assert_eq!(fs::read_to_string(dock.join("notes.txt"))?, "keep");
    assert!(dock.join(".dirac/config.json").exists());
    assert!(api.is_dir());
    #[cfg(unix)]
    assert_eq!(fs::read_link(dock.join("manual-link"))?, api);
    let mut folders_document: Value = serde_json::from_slice(&fs::read(&workspace)?)?;
    folders_document["zwd"]["mode"] = serde_json::json!("folders");
    fs::write(&workspace, folders_document.to_string())?;
    fs::remove_file(dock.join("web"))?;
    let folders_sync = run_with_home_and_cwd(&["sync", &arg], temp.path(), temp.path())?;
    assert!(folders_sync.status.success());
    assert!(dock.join("web").is_dir());
    fs::remove_dir(&web)?;
    let deleted = run_with_home_and_cwd(&["delete", &arg, "--force"], temp.path(), temp.path())?;
    assert!(deleted.status.success());
    assert!(api.is_dir());
    Ok(())
}

#[test]
fn sync_folders_without_dock_is_a_noop_and_requires_reference_outside_dock()
-> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let workspace = temp.path().join("work.code-workspace");
    fs::write(&workspace, r#"{"folders":[],"zwd":{"mode":"folders"}}"#)?;
    let arg = workspace.to_string_lossy();
    let output = run_with_home_and_cwd(&["sync", &arg], temp.path(), temp.path())?;
    assert!(output.status.success());
    assert!(stdout_line(&output).contains("no dock"));
    let missing = run_with_home_and_cwd(&["sync"], temp.path(), temp.path())?;
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("workspace"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn delete_removes_extras_without_following_any_symlinks() -> Result<(), Box<dyn Error>> {
    let temp = tempdir()?;
    let project = temp.path().join("api");
    fs::create_dir(&project)?;
    fs::write(project.join("sentinel"), "keep")?;
    let workspace = temp.path().join("work.code-workspace");
    fs::write(
        &workspace,
        serde_json::json!({"folders":[{"path":project}],"zwd":{"mode":"symlink"}}).to_string(),
    )?;
    let dock = open_dock(&workspace, temp.path(), temp.path())?;
    fs::create_dir(dock.join(".dirac"))?;
    fs::write(dock.join(".dirac/config"), "remove")?;
    std::os::unix::fs::symlink(&project, dock.join("extra"))?;
    std::os::unix::fs::symlink(&project, dock.join(".dirac/nested-link"))?;
    let arg = workspace.to_string_lossy();
    let preview = run_with_home_and_cwd(&["delete", &arg], temp.path(), temp.path())?;
    assert!(preview.status.success());
    assert!(dock.join(".dirac/config").exists());
    let inside = run_with_home_and_cwd(
        &["delete", &arg, "--force"],
        temp.path(),
        &dock.join(".dirac"),
    )?;
    assert!(!inside.status.success());
    assert!(workspace.exists());
    let deleted = run_with_home_and_cwd(&["delete", &arg, "--force"], temp.path(), temp.path())?;
    assert!(
        deleted.status.success(),
        "{}",
        String::from_utf8_lossy(&deleted.stderr)
    );
    assert!(!dock.exists());
    assert!(!workspace.exists());
    assert_eq!(fs::read_to_string(project.join("sentinel"))?, "keep");
    Ok(())
}

#[cfg(unix)]
#[test]
fn dock_conflicts_and_invalid_ownership_fail_without_mutation() -> Result<(), Box<dyn Error>> {
    for case in [
        "target",
        "directory",
        "collision",
        "lock-symlink",
        "root-symlink",
        "owner",
        "traversal",
        "duplicate",
        "relative",
        "version",
    ] {
        let temp = tempdir()?;
        let api = temp.path().join("api");
        let web = temp.path().join("web");
        fs::create_dir(&api)?;
        fs::create_dir(&web)?;
        fs::write(api.join("sentinel"), "keep")?;
        let workspace = temp.path().join("work.code-workspace");
        let document = serde_json::json!({"folders":[{"path":api}],"zwd":{"mode":"symlink"}});
        fs::write(&workspace, document.to_string())?;
        let dock = open_dock(&workspace, temp.path(), temp.path())?;
        let lock_path = dock.join(".zwd-lock.json");
        let mut lock: Value = serde_json::from_slice(&fs::read(&lock_path)?)?;
        match case {
            "target" => {
                fs::remove_file(dock.join("api"))?;
                std::os::unix::fs::symlink(&web, dock.join("api"))?;
            }
            "directory" => {
                fs::remove_file(dock.join("api"))?;
                fs::create_dir(dock.join("api"))?;
            }
            "collision" => {
                fs::write(
                    &workspace,
                    serde_json::json!({"folders":[{"path":web}],"zwd":{"mode":"symlink"}})
                        .to_string(),
                )?;
                std::os::unix::fs::symlink(&web, dock.join("web"))?;
            }
            "lock-symlink" => {
                let outside = temp.path().join("external-lock");
                fs::rename(&lock_path, &outside)?;
                std::os::unix::fs::symlink(outside, &lock_path)?;
            }
            "root-symlink" => {
                let outside = temp.path().join("external-dock");
                fs::rename(&dock, &outside)?;
                std::os::unix::fs::symlink(outside, &dock)?;
            }
            "owner" => {
                lock["workspace_path"] =
                    serde_json::json!(temp.path().join("other.code-workspace"));
            }
            "traversal" => {
                lock["links"][0]["name"] = serde_json::json!("../api");
            }
            "duplicate" => {
                let link = lock["links"][0].clone();
                lock["links"].as_array_mut().unwrap().push(link);
            }
            "relative" => {
                lock["links"][0]["target"] = serde_json::json!("../api");
            }
            "version" => {
                lock["version"] = serde_json::json!(2);
            }
            _ => unreachable!(),
        }
        if matches!(
            case,
            "owner" | "traversal" | "duplicate" | "relative" | "version"
        ) {
            fs::write(&lock_path, lock.to_string())?;
        }
        let before = fs::read(&lock_path)?;
        let workspace_before = fs::read(&workspace)?;
        let arg = workspace.to_string_lossy();
        for command in ["sync", "open"] {
            let output = run_with_home_and_cwd(&[command, &arg], temp.path(), temp.path())?;
            assert!(
                !output.status.success(),
                "{case}: {command} unexpectedly succeeded"
            );
            assert_eq!(fs::read(&lock_path)?, before, "{case}");
            assert_eq!(fs::read(&workspace)?, workspace_before);
            assert_eq!(fs::read_to_string(api.join("sentinel"))?, "keep");
        }
        if !matches!(case, "target" | "directory" | "collision") {
            let output =
                run_with_home_and_cwd(&["delete", &arg, "--force"], temp.path(), temp.path())?;
            assert!(!output.status.success(), "{case}: unsafe delete");
            assert_eq!(fs::read(&lock_path)?, before);
        }
        assert!(dock.join("api").symlink_metadata().is_ok());
    }
    Ok(())
}
