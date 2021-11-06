use std::path::{Path, PathBuf};

use notify::{
    Watcher, 
    RecommendedWatcher,
    RecursiveMode,
    EventKind,
    Event,
    event,
    // ErrorKind,
};
use tokio::runtime::{Handle};
use tokio::sync::mpsc::{
    self
};

use crate::error;

fn watch_directories(watcher: &mut RecommendedWatcher, path: PathBuf) -> error::Result<u32> {
    let entries = std::fs::read_dir(&path)?;
    let mut rtn: u32 = 0;

    for ent in entries {
        let ent = ent?;
        let ent_path = ent.path();

        if ent_path.is_dir() {
            rtn += watch_directories(watcher, ent_path)?
        }
    }

    watcher.watch(&path, RecursiveMode::NonRecursive)?;
    rtn += 1;

    Ok(rtn)
}

fn handle_event(watcher: &mut RecommendedWatcher, evt: Event) -> error::Result<()> {
    println!("new watcher event: {:?}", evt);

    match evt.kind {
        EventKind::Create(create_kind) => match create_kind {
            event::CreateKind::Folder => {
                println!("adding folders to watch. count: {}", evt.paths.len());

                for path in evt.paths {
                    match watcher.watch(&path, RecursiveMode::NonRecursive) {
                        Ok(()) => {},
                        Err(e) => {
                            println!("failed to add folder to watcher.\npath: \"{}\"\nerror: {:?}", path.display(), e)
                        }
                    }
                }

                println!("finished adding directories");
            },
            _ => {}
        },
        EventKind::Remove(remove_kind) => match remove_kind {
            event::RemoveKind::Folder => {
                println!("removing folders to watch. count: {}", evt.paths.len());

                for path in evt.paths {
                    match watcher.unwatch(&path) {
                        Ok(()) => {},
                        Err(e) => println!("error when unwatching path.\npath: \"{}\"\nerror: {:?}", path.display(), e)
                    }
                }

                println!("finished removing directories");
            },
            _ => {}
        }
        _ => {}
    }

    Ok(())
}

/**
 * we can use a handle from a tokio runtime to spawn tasks.
 * since the watcher creates its own thread to handle everything
 * the context is not available to use there so the handle will
 * be given instead since it is just a reference counter.
 */
pub async fn watch(path: PathBuf) -> error::Result<()> {
    let (tx, mut rx) = mpsc::channel(24);
    let rt = Handle::current();

    let mut watcher = RecommendedWatcher::new(move |evt| {
        // from the tokio guides, we need to spawn a new sender
        // that can be given to the task since it is a one
        // time use
        let tx2 = tx.clone();

        rt.spawn(async move {
            if let Err(e) = tx2.try_send(evt) {
                match e {
                    mpsc::error::TrySendError::Full(_v) => {
                        println!("sender queue is full");
                    },
                    mpsc::error::TrySendError::Closed(_v) => {
                        println!("sender is closed");
                    }
                }
            }
        });
    })?;
    let total = watch_directories(&mut watcher, path)?;
    println!("total directories watched: {}", total);

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => handle_event(&mut watcher, event)?,
            Err(e) => println!("watch error: {:?}", e)
        }
    }

    Ok(())
}