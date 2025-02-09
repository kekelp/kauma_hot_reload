mod rebuild;
pub use rebuild::*;

use notify_debouncer_full::{notify::*, new_debouncer, DebounceEventResult};
use std::time::Duration;

fn main() -> Result<()> {
    
    let _ = rebuild();
    
    let mut debouncer = new_debouncer(Duration::from_secs_f32(0.5), None, |result: DebounceEventResult| {
        match result {
            Err(e) => println!("Error watching for code changes: {:?}", e),
            Ok(_) => {
                let _ = rebuild();
            }
        }
    })?;
    
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer.watch("src", RecursiveMode::Recursive)?;
    
    loop {}
}