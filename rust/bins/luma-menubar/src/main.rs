mod appkit;
mod instance_lock;
mod launcher;
mod model;
mod worker;

use appkit::MenuController;
use block2::RcBlock;
use instance_lock::InstanceLock;
use model::MenuSnapshot;
use objc2_app_kit::NSApplication;
use objc2_foundation::{MainThreadMarker, NSOperationQueue};
use std::sync::mpsc;

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
    let application_ptr = (&*application as *const NSApplication) as usize;
    let (action_tx, action_rx) = mpsc::channel();
    let mut controller = MenuController::new(mtm, action_tx);
    let controller_ptr = (&mut controller as *mut MenuController) as usize;
    worker::spawn_worker(
        action_rx,
        Box::new(move |snapshot: MenuSnapshot| {
            dispatch_main(move || {
                // SAFETY: controller remains alive for the duration of NSApplication::run.
                unsafe { (&mut *(controller_ptr as *mut MenuController)).apply_snapshot(snapshot) };
            });
        }),
        Box::new(move || {
            dispatch_main(move || {
                // SAFETY: NSApplication is retained by main until its event loop returns.
                unsafe { (&*(application_ptr as *const NSApplication)).terminate(None) };
            });
        }),
        worker::initial_snapshot(),
    );
    appkit::request_refresh();
    application.run();
}

type MainTask = Box<dyn FnOnce() + Send + 'static>;

fn dispatch_main(task: impl FnOnce() + Send + 'static) {
    let task = std::sync::Mutex::new(Some(Box::new(task) as MainTask));
    let block = RcBlock::new(move || {
        if let Ok(mut task) = task.lock() {
            if let Some(task) = task.take() {
                task();
            }
        }
    });
    let queue = NSOperationQueue::mainQueue();
    // SAFETY: the captured task is Send and is consumed exactly once by the main queue block.
    unsafe { queue.addOperationWithBlock(&block) };
}
