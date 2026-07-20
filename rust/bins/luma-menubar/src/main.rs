mod appkit;
mod instance_lock;
mod launcher;
mod model;
mod worker;

use appkit::MenuController;
use instance_lock::InstanceLock;
use objc2_app_kit::NSApplication;
use objc2_foundation::MainThreadMarker;
use std::sync::{mpsc, Arc, Mutex};

fn main() {
    let support_dir = luma_storage::luma_next_support_dir().unwrap_or_else(|error| {
        eprintln!("Luma menu bar unavailable: {error}");
        std::process::exit(1);
    });
    let _instance_lock =
        InstanceLock::acquire(&support_dir.join("menubar.lock")).unwrap_or_else(|error| {
            eprintln!("Luma menu bar: {error}");
            std::process::exit(0);
        });
    let mtm = MainThreadMarker::new().expect("luma-menubar must start on the main thread");
    let application = NSApplication::sharedApplication(mtm);
    let (action_tx, action_rx) = mpsc::channel();
    let initial_snapshot = worker::initial_snapshot();
    let snapshots = Arc::new(Mutex::new(initial_snapshot.clone()));
    let controller =
        MenuController::new(mtm, action_tx, snapshots.clone(), initial_snapshot.clone());
    let worker = worker::spawn_worker(action_rx, snapshots, initial_snapshot);
    appkit::request_refresh();
    application.run();
    drop(controller);
    let _ = worker.join();
}
