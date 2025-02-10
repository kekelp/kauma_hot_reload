use std::path::PathBuf;

pub const KAUMA_HOT_BUILD_DIR: &str = "kauma_hot_reload";
pub const KAUMA_ENV_VAR: &str = "KAUMA_HOT_RELOAD_BUILD";
pub const KAUMA_SHARED_LIB_NAME: &str = "kauma_shared_lib";

pub fn cargo_target_dir() -> PathBuf {
    return std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target"));
}
