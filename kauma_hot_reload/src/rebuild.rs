use std::env;
use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process::{Command, exit};
use std::os::unix::fs::symlink;
use toml::{de, value::{Table, Value}};

pub const KAUMA_BUILD_DIR: &str = "kauma_hot_reload";
pub const KAUMA_ENV_VAR: &str = "KAUMA_HOT_RELOAD_BUILD";

pub fn cargo_target_dir() -> PathBuf {
    return env::var("CARGO_TARGET_DIR").map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target"));
}

pub fn rebuild() -> io::Result<()> {
    // Create build directory if it doesn't exist
    let kauma_build_dir = cargo_target_dir().join(KAUMA_BUILD_DIR);
    
    if !fs::metadata(kauma_build_dir.clone()).is_ok() {
        fs::create_dir(kauma_build_dir.clone())?;
    }

    // Create a symlink from to the "src" folder
    let current_dir = env::current_dir()?;
    let src_dir = current_dir.join("src");
    let hot_stuff_src_dir = kauma_build_dir.join("src");
    
    if !fs::metadata(&hot_stuff_src_dir).is_ok() {
        symlink(src_dir, hot_stuff_src_dir.clone())?;
    }

    // Copy Cargo.toml from the current directory to the build dit
    let cargo_toml_path = current_dir.join("Cargo.toml");
    let hot_stuff_cargo_toml_path = kauma_build_dir.join("Cargo.toml");
    fs::copy(cargo_toml_path, &hot_stuff_cargo_toml_path)?;


    // Parse the Cargo.toml file in the build dir
    let mut cargo_toml_file = File::open(&hot_stuff_cargo_toml_path)?;
    let mut cargo_toml_content = String::new();
    cargo_toml_file.read_to_string(&mut cargo_toml_content)?;

    let mut parsed_toml: Table = de::from_str(&cargo_toml_content).unwrap();

    // Fix path dependencies and add `[lib]` section
    fix_path_dependencies(&mut parsed_toml);
    add_lib_section(&mut parsed_toml);

    // Write the modified Cargo.toml back in the build dir
    let mut hot_stuff_cargo_toml = File::create(&hot_stuff_cargo_toml_path)?;
    let modified_toml = toml::to_string(&parsed_toml).unwrap();
    hot_stuff_cargo_toml.write_all(modified_toml.as_bytes())?;

    // Run `cargo build` in "hot_stuff" with HOT_RELOAD_BUILD=true
    let status = Command::new("cargo")
        .env(KAUMA_ENV_VAR, "true")
        .current_dir(kauma_build_dir)
        .arg("build")
        .status()?;

    if !status.success() {
        exit(1);
    }

    Ok(())
}

// Function to fix up path dependencies by adding a ".." prefix
fn fix_path_dependencies(toml_table: &mut Table) {
    if let Some(dependencies) = toml_table.get_mut("dependencies") {
        if let Some(dep_table) = dependencies.as_table_mut() {
            for (_, value) in dep_table.iter_mut() {
                if let Some(path) = value.get_mut("path") {
                    if let Some(path_str) = path.as_str() {
                        if !path_str.starts_with("..") {
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

// Function to add the `[lib]` section to the Cargo.toml
fn add_lib_section(toml_table: &mut Table) {
    let mut lib_section = Table::new();
    lib_section.insert("crate-type".to_string(), Value::Array(vec![Value::String("cdylib".to_string())]));
    lib_section.insert("path".to_string(), Value::String("src/main.rs".to_string()));

    toml_table.insert("lib".to_string(), Value::Table(lib_section));
}
