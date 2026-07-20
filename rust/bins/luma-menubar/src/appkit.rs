use crate::model::{
    ActionStatus, LoginItemState, MenuAction, MenuSnapshot, SharedMenuSnapshot, WindowsStatus,
    WordbookStatus,
};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSImage, NSLineCapStyle, NSMenu, NSMenuDelegate, NSMenuItem,
    NSStatusBar, NSStatusItem,
};
use objc2_foundation::{ns_string, NSObjectProtocol, NSPoint, NSSize, NSString};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

const TAG_REVIEW_DUE: isize = 10;
const TAG_OPEN_LUMA: isize = 11;
const TAG_OPEN_SETTINGS: isize = 12;
const TAG_REFRESH: isize = 13;
const TAG_LOGIN: isize = 14;
const TAG_QUIT: isize = 15;

static ACTIONS: OnceLock<Mutex<Option<Sender<MenuAction>>>> = OnceLock::new();

pub struct MenuTargetIvars {
    menu: Retained<NSMenu>,
    snapshots: SharedMenuSnapshot,
}

pub fn request_refresh() {
    send_action(MenuAction::Refresh);
}

fn send_action(action: MenuAction) {
    if let Some(sender) = ACTIONS
        .get()
        .and_then(|slot| slot.lock().ok().and_then(|g| g.clone()))
    {
        let _ = sender.send(action);
    }
}

define_class!(
    // SAFETY: MenuTarget has NSObject as its superclass and its main-thread-only ivars are
    // retained for the lifetime of the AppKit menu.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = MenuTargetIvars]
    pub struct MenuTarget;

    // SAFETY: NSObjectProtocol has no additional requirements.
    unsafe impl NSObjectProtocol for MenuTarget {}

    impl MenuTarget {
        #[unsafe(method(performMenuAction:))]
        fn perform_menu_action(&self, sender: Option<&NSMenuItem>) {
            let Some(sender) = sender else { return };
            if let Some(window_id) = sender.representedObject().and_then(|object| {
                object
                    .downcast::<NSString>()
                    .ok()
                    .map(|value| value.to_string())
            }) {
                send_action(MenuAction::FocusWindow(window_id));
                return;
            }
            let action = match sender.tag() {
                TAG_REVIEW_DUE => MenuAction::ReviewDue,
                TAG_OPEN_LUMA => MenuAction::OpenLuma,
                TAG_OPEN_SETTINGS => MenuAction::OpenSettings,
                TAG_REFRESH => MenuAction::Refresh,
                TAG_LOGIN => MenuAction::ToggleLaunchAtLogin,
                TAG_QUIT => MenuAction::Quit,
                _ => return,
            };
            let quit = matches!(&action, MenuAction::Quit);
            send_action(action);
            if quit {
                objc2_app_kit::NSApplication::sharedApplication(self.mtm()).terminate(None);
            }
        }
    }

    // SAFETY: NSMenuDelegate has no required methods and MenuTarget is main-thread-only.
    unsafe impl NSMenuDelegate for MenuTarget {
        #[unsafe(method(menuWillOpen:))]
        #[allow(non_snake_case)]
        fn menuWillOpen(&self, _menu: &NSMenu) {
            self.apply_latest_snapshot();
            send_action(MenuAction::Refresh);
        }
    }
);

impl MenuTarget {
    fn new(
        mtm: MainThreadMarker,
        menu: Retained<NSMenu>,
        snapshots: SharedMenuSnapshot,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MenuTargetIvars { menu, snapshots });
        // SAFETY: NSObject's init is the designated initializer for this empty subclass.
        unsafe { msg_send![super(this), init] }
    }

    fn apply_latest_snapshot(&self) {
        let snapshot = match self.ivars().snapshots.lock() {
            Ok(snapshot) => snapshot.clone(),
            Err(_) => return,
        };
        render_menu(&self.ivars().menu, self.mtm(), self, &snapshot);
    }
}

pub struct MenuController {
    mtm: MainThreadMarker,
    _status_item: Retained<NSStatusItem>,
    _status_image: Retained<NSImage>,
    menu: Retained<NSMenu>,
    _target: Retained<MenuTarget>,
}

impl MenuController {
    #[allow(deprecated)]
    pub fn new(
        mtm: MainThreadMarker,
        actions: Sender<MenuAction>,
        snapshots: SharedMenuSnapshot,
        initial_snapshot: MenuSnapshot,
    ) -> Self {
        let _ = ACTIONS.set(Mutex::new(Some(actions)));
        let menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!("Luma"));
        let target = MenuTarget::new(mtm, menu.clone(), snapshots);
        menu.setDelegate(Some(ProtocolObject::from_ref(&*target)));

        let status_bar = NSStatusBar::systemStatusBar();
        // NSVariableStatusItemLength is -1.0 in AppKit.
        let status_item = status_bar.statusItemWithLength(-1.0);
        let status_image = orbit_image(mtm);
        status_item.setImage(Some(&status_image));
        let tooltip = NSString::from_str("Luma");
        status_item.setToolTip(Some(&tooltip));
        status_item.setMenu(Some(&menu));

        let mut controller = Self {
            mtm,
            _status_item: status_item,
            _status_image: status_image,
            menu,
            _target: target,
        };
        controller.apply_snapshot(initial_snapshot);
        controller
    }

    #[allow(deprecated)]
    pub fn apply_snapshot(&mut self, snapshot: MenuSnapshot) {
        render_menu(&self.menu, self.mtm, &self._target, &snapshot);
    }
}

impl Drop for MenuController {
    fn drop(&mut self) {
        if let Some(slot) = ACTIONS.get() {
            if let Ok(mut sender) = slot.lock() {
                *sender = None;
            }
        }
    }
}

fn render_menu(menu: &NSMenu, mtm: MainThreadMarker, target: &MenuTarget, snapshot: &MenuSnapshot) {
    menu.removeAllItems();
    menu.addItem(&section("Wordbook", mtm));
    let review_enabled = matches!(
        &snapshot.wordbook,
        WordbookStatus::Ready { due, .. } if *due > 0
    );
    match &snapshot.wordbook {
        WordbookStatus::NotConfigured => {
            menu.addItem(&item("Wordbook not configured", 0, false, mtm, target))
        }
        WordbookStatus::Unavailable(message) => menu.addItem(&item(
            &format!("Wordbook unavailable: {message}"),
            0,
            false,
            mtm,
            target,
        )),
        WordbookStatus::Ready {
            due,
            reviewed_today,
            goal,
        } => {
            let summary = if *due == 0 {
                format!("All caught up · {reviewed_today}/{goal} today")
            } else {
                format!("{due} due · {reviewed_today}/{goal} today")
            };
            menu.addItem(&item(&summary, 0, false, mtm, target));
        }
        WordbookStatus::Stale {
            due,
            reviewed_today,
            goal,
            reason,
        } => {
            menu.addItem(&item(
                &format!("Wordbook stale: {reason}"),
                0,
                false,
                mtm,
                target,
            ));
            menu.addItem(&item(
                &format!("Last known: {due} due · {reviewed_today}/{goal} today"),
                0,
                false,
                mtm,
                target,
            ));
        }
    }
    menu.addItem(&item(
        "Review due in Luma…",
        TAG_REVIEW_DUE,
        review_enabled,
        mtm,
        target,
    ));
    menu.addItem(&separator(mtm));
    menu.addItem(&section("Windows", mtm));
    match &snapshot.windows {
        WindowsStatus::PermissionRequired(message) | WindowsStatus::Unavailable(message) => {
            menu.addItem(&item(message, 0, false, mtm, target));
        }
        WindowsStatus::Ready(windows) if windows.is_empty() => {
            menu.addItem(&item("No visible windows", 0, false, mtm, target));
        }
        WindowsStatus::Ready(windows) => {
            for window in windows {
                menu.addItem(&window_item(window, mtm, target));
            }
        }
        WindowsStatus::Stale { windows, reason } => {
            menu.addItem(&item(
                &format!("Windows stale: {reason}"),
                0,
                false,
                mtm,
                target,
            ));
            for window in windows {
                menu.addItem(&window_item(window, mtm, target));
            }
        }
    }
    if let Some(warning) = &snapshot.global_warning {
        menu.addItem(&separator(mtm));
        menu.addItem(&item(&format!("Warning: {warning}"), 0, false, mtm, target));
    }
    if let Some(action) = &snapshot.last_action {
        let message = match action {
            ActionStatus::Succeeded(message) => format!("Last action: {message}"),
            ActionStatus::Failed(message) => format!("Action failed: {message}"),
        };
        menu.addItem(&item(&message, 0, false, mtm, target));
    }
    menu.addItem(&separator(mtm));
    let cli_title = match &snapshot.cli {
        crate::model::CliStatus::Available => "Open Luma".to_string(),
        crate::model::CliStatus::Unavailable(reason) => format!("Luma unavailable: {reason}"),
    };
    menu.addItem(&item(
        &cli_title,
        TAG_OPEN_LUMA,
        snapshot.cli.is_available(),
        mtm,
        target,
    ));
    menu.addItem(&item(
        "Open /settings in Luma",
        TAG_OPEN_SETTINGS,
        snapshot.cli.is_available(),
        mtm,
        target,
    ));
    menu.addItem(&item("Refresh", TAG_REFRESH, true, mtm, target));
    menu.addItem(&item(
        &login_item_title(&snapshot.login_item),
        TAG_LOGIN,
        !matches!(snapshot.login_item, LoginItemState::NotFound),
        mtm,
        target,
    ));
    menu.addItem(&item("Quit Luma Menu Bar", TAG_QUIT, true, mtm, target));
    menu.update();
}

fn window_menu_title(app_name: &str, title: &str) -> String {
    if title == "Untitled" || title.trim().is_empty() {
        app_name.to_string()
    } else {
        format!("{app_name} — {title}")
    }
}

fn window_item(
    window: &luma_application::WindowEntry,
    mtm: MainThreadMarker,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    let title = window_menu_title(&window.app_name, &window.title);
    let item = item(&title, 0, true, mtm, target);
    let represented = NSString::from_str(&window.id);
    // SAFETY: representedObject accepts an Objective-C object and the retained NSString lives
    // through the menu item, which retains its represented object.
    unsafe { item.setRepresentedObject(Some(&represented)) };
    item
}

fn login_item_title(state: &LoginItemState) -> String {
    match state {
        LoginItemState::NotRegistered => "Launch at Login".into(),
        LoginItemState::Enabled => "Launch at Login ✓".into(),
        LoginItemState::RequiresApproval => "Launch at Login · Approval required".into(),
        LoginItemState::NotFound => "Launch at Login · Bundle unavailable".into(),
        LoginItemState::Unavailable(reason) => format!("Launch at Login · {reason}"),
    }
}

#[allow(deprecated)]
fn orbit_image(mtm: MainThreadMarker) -> Retained<NSImage> {
    let image = NSImage::initWithSize(mtm.alloc(), NSSize::new(18.0, 18.0));
    image.lockFocus();
    let color = NSColor::blackColor();
    color.setStroke();
    for (start, end, control1, control2) in [
        (
            NSPoint::new(4.85, 13.85),
            NSPoint::new(13.85, 9.55),
            NSPoint::new(2.2, 11.1),
            NSPoint::new(3.7, 15.1),
        ),
        (
            NSPoint::new(13.15, 13.85),
            NSPoint::new(4.15, 9.55),
            NSPoint::new(15.8, 11.1),
            NSPoint::new(14.3, 15.1),
        ),
        (
            NSPoint::new(6.5, 4.85),
            NSPoint::new(15.5, 13.85),
            NSPoint::new(8.9, 2.2),
            NSPoint::new(4.9, 3.7),
        ),
    ] {
        let path = NSBezierPath::bezierPath();
        path.moveToPoint(start);
        path.curveToPoint_controlPoint1_controlPoint2(end, control1, control2);
        path.setLineWidth(1.9);
        path.setLineCapStyle(NSLineCapStyle::Round);
        path.stroke();
    }
    image.unlockFocus();
    image.setTemplate(true);
    image
}

fn section(title: &str, mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let title = NSString::from_str(title);
    NSMenuItem::sectionHeaderWithTitle(&title, mtm)
}

fn separator(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    NSMenuItem::separatorItem(mtm)
}

fn item(
    title: &str,
    tag: isize,
    enabled: bool,
    mtm: MainThreadMarker,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let title = NSString::from_str(title);
    item.setTitle(&title);
    item.setTag(tag);
    item.setEnabled(enabled);
    // SAFETY: MenuTarget implements the selector with the NSMenuItem sender signature.
    unsafe {
        item.setTarget(Some(target.as_ref()));
        item.setAction(Some(sel!(performMenuAction:)));
    }
    item
}

#[cfg(test)]
mod tests {
    use super::window_menu_title;

    #[test]
    fn empty_window_titles_fall_back_to_the_app_name() {
        assert_eq!(
            window_menu_title("Google Chrome", "Untitled"),
            "Google Chrome"
        );
        assert_eq!(window_menu_title("Finder", ""), "Finder");
        assert_eq!(
            window_menu_title("Safari", "Luma docs"),
            "Safari — Luma docs"
        );
    }
}
