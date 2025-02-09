use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process::{Command, exit};
use std::os::unix::fs::symlink;
use toml::{de, value::{Table, Value}};

pub fn rebuild() -> io::Result<()> {
    // Create "hot_stuff" directory if it doesn't exist
    let hot_stuff_dir = "hot_stuff";
    if !fs::metadata(hot_stuff_dir).is_ok() {
        fs::create_dir(hot_stuff_dir)?;
        println!("Created directory: {}", hot_stuff_dir);
    }

    // Create a symlink from "hot_stuff/src" to the current crate's "src" folder
    let current_dir = env::current_dir()?;
    let src_dir = current_dir.join("src");
    let hot_stuff_src_dir = hot_stuff_dir.to_string() + "/src";
    
    if !fs::metadata(&hot_stuff_src_dir).is_ok() {
        symlink(src_dir, hot_stuff_src_dir.clone())?;
        println!("Created symlink");
    }

    // Copy Cargo.toml from the current directory to hot_stuff
    let cargo_toml_path = current_dir.join("Cargo.toml");
    let hot_stuff_cargo_toml_path = hot_stuff_dir.to_string() + "/Cargo.toml";
    fs::copy(cargo_toml_path, &hot_stuff_cargo_toml_path)?;

    println!("Copied Cargo.toml to hot_stuff");

    // Parse the Cargo.toml file in "hot_stuff"
    let mut cargo_toml_file = File::open(&hot_stuff_cargo_toml_path)?;
    let mut cargo_toml_content = String::new();
    cargo_toml_file.read_to_string(&mut cargo_toml_content)?;

    let mut parsed_toml: Table = de::from_str(&cargo_toml_content).unwrap();

    // Fix path dependencies and add `[lib]` section
    fix_path_dependencies(&mut parsed_toml);
    add_lib_section(&mut parsed_toml);

    // Write the modified Cargo.toml back to hot_stuff
    let mut hot_stuff_cargo_toml = File::create(&hot_stuff_cargo_toml_path)?;
    let modified_toml = toml::to_string(&parsed_toml).unwrap();
    hot_stuff_cargo_toml.write_all(modified_toml.as_bytes())?;

    println!("Modified Cargo.toml in hot_stuff");

    // Run `cargo build` in "hot_stuff" with HOT_RELOAD_BUILD=true
    let status = Command::new("cargo")
        .env("HOT_RELOAD_BUILD", "true")
        .current_dir(hot_stuff_dir)
        .arg("build")
        .status()?;

    if !status.success() {
        eprintln!("Cargo build failed.");
        exit(1);
    }

    println!("Cargo build completed in hot_stuff");

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
                            let new_path = format!("../{}", path_str);
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
