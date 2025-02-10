use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::symlink;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use notify_debouncer_full::{notify::*, new_debouncer, DebounceEventResult};
use std::time::Duration;
use toml::{
    de,
    value::{Table, Value},
};

use kauma_shared_types::*;

pub fn rebuild() -> io::Result<()> {
    // Create build directory if it doesn't exist
    let hot_build_dir = cargo_target_dir().join(KAUMA_HOT_BUILD_DIR);

    if !fs::metadata(hot_build_dir.clone()).is_ok() {
        fs::create_dir(hot_build_dir.clone())?;
    }

    // Create a symlink from to the "src" folder
    let current_dir = env::current_dir()?;
    let src_dir = current_dir.join("src");
    let build_src_dir = hot_build_dir.join("src");

    if !fs::metadata(&build_src_dir).is_ok() {
        symlink(src_dir, build_src_dir.clone())?;
    }

    // Copy Cargo.toml from the current directory to the build dir
    let cargo_toml_path = current_dir.join("Cargo.toml");
    let hot_build_cargo_toml_path = hot_build_dir.join("Cargo.toml");
    fs::copy(cargo_toml_path, &hot_build_cargo_toml_path)?;

    // Parse the main Cargo.toml file
    let mut cargo_toml_file = File::open(&hot_build_cargo_toml_path)?;
    let mut cargo_toml_content = String::new();
    cargo_toml_file.read_to_string(&mut cargo_toml_content)?;

    let mut parsed_toml: Table = de::from_str(&cargo_toml_content).unwrap();

    // Modify the Cargo.toml
    modify_package_name(&mut parsed_toml);
    fix_path_dependencies(&mut parsed_toml);
    add_lib_section(&mut parsed_toml);

    // Write the modified Cargo.toml back in the build dir
    let mut hot_build_cargo_toml = File::create(&hot_build_cargo_toml_path)?;
    let modified_toml = toml::to_string(&parsed_toml).unwrap();
    hot_build_cargo_toml.write_all(modified_toml.as_bytes())?;

    // Run `cargo build` in the build dir with HOT_RELOAD_BUILD=true
    let _status = Command::new("cargo")
        .env(KAUMA_ENV_VAR, "true")
        .current_dir(hot_build_dir)
        .arg("build")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
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
    
    let _ = rebuild();
    
    let debouncer = new_debouncer(Duration::from_secs_f32(0.5), None, |result: DebounceEventResult| {
        match result {
            Err(e) => println!("Error watching for code changes: {:?}", e),
            Ok(_) => {
                let _ = rebuild();
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
    std::thread::spawn(|| {
        watch_and_rebuild();
    });
}

