mod rebuild;
use rebuild::*;

use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher};
use std::{path::Path, sync::mpsc};

fn main() -> Result<()> {
    rebuild();

    let (tx, rx) = mpsc::channel::<Result<Event>>();

    let mut watcher = recommended_watcher(tx)?;

    watcher.watch(Path::new("src"), RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Err(e) => println!("Error watching for code changes: {:?}", e),
            Ok(_) => {
                let _ = rebuild();
            }
        }
    }

    Ok(())
}