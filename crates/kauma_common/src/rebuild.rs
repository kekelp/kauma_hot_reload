use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::symlink;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use cargo_metadata::camino::Utf8PathBuf;
use cargo_metadata::{Metadata, MetadataCommand};
use notify_debouncer_full::{notify::*, new_debouncer, DebounceEventResult};
use std::time::Duration;
use toml::{
    de,
    value::{Table, Value},
};
use crate::*;

pub fn cargo_target_dir() -> PathBuf {
    return std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target"));
}

pub fn get_cargo_target_dirs(metadata: &Metadata) -> HashSet<Utf8PathBuf> {
    let mut target_dirs = HashSet::new();

    for package in &metadata.packages {
        for target in &package.targets {
            if let Some(parent) = target.src_path.parent() {
                target_dirs.insert(parent.to_path_buf());
            }
        }
    }

    target_dirs
}

pub fn rebuild() -> io::Result<()> {
    let metadata = MetadataCommand::new()
        .exec()
        .expect("Failed to retrieve Cargo metadata");

    let project_root = &metadata.workspace_root;

    // Create build directory if it doesn't exist
    let hot_build_dir = metadata.target_directory.join(KAUMA_HOT_BUILD_DIR);

    if !fs::metadata(&hot_build_dir).is_ok() {
        fs::create_dir(&hot_build_dir).expect("1");
    }

    // Create symlinks for all the folders containing code
    // todo: see if this works if a target is in the project root outside of src/
    let target_dirs = get_cargo_target_dirs(&metadata);

    let build_src_dir = hot_build_dir.join("src");

    if build_src_dir.exists() {
        fs::remove_file(&build_src_dir).unwrap();
    }    

    let src_dir = project_root.join("src");

    symlink(&src_dir, &build_src_dir).expect("Couldn't create symbolic link for src directory");

    // for src_dir in target_dirs {
    //     let relative_path = src_dir.strip_prefix(env::current_dir().expect("2")).unwrap_or(&src_dir);
    //     let build_src_dir = hot_build_dir.join(relative_path);

    //     if !build_src_dir.exists() {
    //         fs::create_dir_all(build_src_dir.parent().unwrap()).expect("4");
    //         symlink(&src_dir, &build_src_dir).expect("5");
    //     }
    // }

    // Copy Cargo.toml from the current directory to the build dir
    let cargo_toml_path = project_root.join("Cargo.toml");
    let hot_build_cargo_toml_path = hot_build_dir.join("Cargo.toml");
    fs::copy(cargo_toml_path, &hot_build_cargo_toml_path).expect("6");

    // Parse the main Cargo.toml file
    let mut cargo_toml_file = File::open(&hot_build_cargo_toml_path).expect("7");
    let mut cargo_toml_content = String::new();
    cargo_toml_file.read_to_string(&mut cargo_toml_content).expect("8");

    let mut parsed_toml: Table = de::from_str(&cargo_toml_content).expect("9");

    // Modify the Cargo.toml
    modify_package_name(&mut parsed_toml);
    fix_path_dependencies(&mut parsed_toml);
    add_lib_section(&mut parsed_toml);

    // Write the modified Cargo.toml back in the build dir
    let mut hot_build_cargo_toml = File::create(&hot_build_cargo_toml_path).expect("10");
    let modified_toml = toml::to_string(&parsed_toml).expect("11");
    hot_build_cargo_toml.write_all(modified_toml.as_bytes()).expect("12");

    // Run `cargo build` in the build dir with HOT_RELOAD_BUILD=true
    let _status = Command::new("cargo")
        .env(KAUMA_ENV_VAR, "true")
        .current_dir(hot_build_dir)
        .arg("build")
        // .stdout(std::process::Stdio::null())
        // .stderr(std::process::Stdio::null())
        .status();

    Ok(())
}

fn modify_package_name(toml_table: &mut Table) {
    if let Some(package) = toml_table.get_mut("package") {
        if let Some(package_table) = package.as_table_mut() {
            // Modify the "name" field in the [package] section
            package_table.insert(
                "name".to_string(),
                Value::String(KAUMA_SHARED_LIB_NAME.to_string()),
            );
        }
    }
}

fn fix_path_dependencies(toml_table: &mut Table) {
    if let Some(dependencies) = toml_table.get_mut("dependencies") {
        if let Some(dep_table) = dependencies.as_table_mut() {
            for (_, value) in dep_table.iter_mut() {
                if let Some(path) = value.get_mut("path") {
                    if let Some(path_str) = path.as_str() {
                        // todo: better check
                        if !path_str.starts_with("/") {
                            // todo: this assumes that we're in a regular target/ directory.
                            let new_path = format!("../../{}", path_str);
                            *path = Value::String(new_path);
                        }
                    }
                }
            }
        }
    }
}

fn add_lib_section(toml_table: &mut Table) {
    let mut lib_section = Table::new();
    lib_section.insert(
        "crate-type".to_string(),
        Value::Array(vec![Value::String("cdylib".to_string())]),
    );
    lib_section.insert("path".to_string(), Value::String("src/main.rs".to_string()));

    toml_table.insert("lib".to_string(), Value::Table(lib_section));
}

fn watch_and_rebuild() {

    println!("Rebuilding hot reload functions and watching for changes...");
    
    let res = rebuild();
    if let Err(e) = res {
        eprintln!("Couldn't rebuild hot-reloaded functions: {}", e);
    }

    let debouncer = new_debouncer(Duration::from_secs_f32(0.5), None, |result: DebounceEventResult| {
        match result {
            Err(e) => println!("Error watching for code changes: {:?}", e),
            Ok(_) => {
                let res = rebuild();
                if let Err(e) = res {
                    eprintln!("Couldn't rebuild hot-reloaded functions: {}", e);
                }
            }
        }
    });
    
    let mut debouncer = match debouncer {
        Ok(debouncer) => debouncer,
        Err(e) => {
            println!("Error watching for code changes: {:?}", e);
            return;
        },
    };

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    let res = debouncer.watch("src", RecursiveMode::Recursive);

    if let Err(e) = res {
        println!("Error watching for code changes: {:?}", e);
    }
    
    loop {}
}

pub static REBUILD_PROCESS_STARTED: AtomicBool = AtomicBool::new(false);

pub fn spawn_rebuild_process() {
    if REBUILD_PROCESS_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Doing this in a thread isn't very good, because if the user tries to profile its process, the directory watcher thread will show up inside it.
    // The rebuilds are launched as separate `cargo build` processes, so they don't contribute to this problem.
    // At least on Unix, it should be possible to do this as a child process, but that's probably not worth the trouble.
    let thread_name = "kauma_hot_reload change watcher".to_string();
    std::thread::Builder::new().name(thread_name).spawn(|| {
        watch_and_rebuild();
    }).unwrap();
}

