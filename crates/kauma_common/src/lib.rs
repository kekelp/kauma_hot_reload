mod rebuild;

use std::path::PathBuf;

pub const KAUMA_HOT_BUILD_DIR: &str = "kauma_hot_reload_target";
pub const KAUMA_ENV_VAR: &str = "KAUMA_HOT_RELOAD_BUILD";
pub const KAUMA_SHARED_LIB_NAME: &str = "kauma_hot_reload_shared_lib";

pub use rebuild::spawn_rebuild_process;
pub use rebuild::rebuild;
pub use rebuild::cargo_target_dir;

pub use libloading::Library as LibLoadingLibrary;
pub use libloading::Symbol as LibLoadingSymbol;
