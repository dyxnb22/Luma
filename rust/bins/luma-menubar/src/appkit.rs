use crate::model::{MenuAction, MenuSnapshot, WindowsStatus, WordbookStatus};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, MainThreadMarker, MainThreadOnly};
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
const TAG_FOCUS_BASE: isize = 1000;

static ACTIONS: OnceLock<Mutex<Option<Sender<MenuAction>>>> = OnceLock::new();

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
    // SAFETY: MenuTarget has NSObject as its superclass and has no ivars or Drop implementation.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct MenuTarget;

    // SAFETY: NSObjectProtocol has no additional requirements.
    unsafe impl NSObjectProtocol for MenuTarget {}

    impl MenuTarget {
        #[unsafe(method(performMenuAction:))]
        fn perform_menu_action(&self, sender: Option<&NSMenuItem>) {
            let Some(sender) = sender else { return };
            let action = match sender.tag() {
                TAG_REVIEW_DUE => MenuAction::ReviewDue,
                TAG_OPEN_LUMA => MenuAction::OpenLuma,
                TAG_OPEN_SETTINGS => MenuAction::OpenSettings,
                TAG_REFRESH => MenuAction::Refresh,
                TAG_LOGIN => MenuAction::ToggleLaunchAtLogin,
                TAG_QUIT => MenuAction::Quit,
                tag if tag >= TAG_FOCUS_BASE => MenuAction::FocusWindow((tag - TAG_FOCUS_BASE) as usize),
                _ => return,
            };
            send_action(action);
        }
    }

    // SAFETY: NSMenuDelegate has no required methods and MenuTarget is main-thread-only.
    unsafe impl NSMenuDelegate for MenuTarget {
        #[unsafe(method(menuWillOpen:))]
        #[allow(non_snake_case)]
        fn menuWillOpen(&self, _menu: &NSMenu) {
            send_action(MenuAction::Refresh);
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: NSObject's init is the designated initializer for this empty subclass.
        unsafe { msg_send![super(this), init] }
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
    pub fn new(mtm: MainThreadMarker, actions: Sender<MenuAction>) -> Self {
        let target = MenuTarget::new(mtm);
        let _ = ACTIONS.set(Mutex::new(Some(actions)));
        let menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!("Luma"));
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
        controller.apply_snapshot(MenuSnapshot::default());
        controller
    }

    #[allow(deprecated)]
    pub fn apply_snapshot(&mut self, snapshot: MenuSnapshot) {
        self.menu.removeAllItems();
        self.menu.addItem(&section("Wordbook", self.mtm));
        let review_enabled = matches!(
            &snapshot.wordbook,
            WordbookStatus::Ready { due, .. } if *due > 0
        );
        match &snapshot.wordbook {
            WordbookStatus::NotConfigured => {
                self.menu.addItem(&item(
                    "Wordbook not configured",
                    0,
                    false,
                    self.mtm,
                    &self._target,
                ));
            }
            WordbookStatus::Unavailable(message) => {
                self.menu.addItem(&item(
                    &format!("Wordbook unavailable: {message}"),
                    0,
                    false,
                    self.mtm,
                    &self._target,
                ));
            }
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
                self.menu
                    .addItem(&item(&summary, 0, false, self.mtm, &self._target));
            }
        }
        self.menu.addItem(&item(
            "Review due in Luma…",
            TAG_REVIEW_DUE,
            review_enabled,
            self.mtm,
            &self._target,
        ));
        self.menu.addItem(&separator(self.mtm));
        self.menu.addItem(&section("Windows", self.mtm));
        match &snapshot.windows {
            WindowsStatus::Unavailable(message) => {
                self.menu
                    .addItem(&item(message, 0, false, self.mtm, &self._target));
            }
            WindowsStatus::Ready(windows) if windows.is_empty() => {
                self.menu.addItem(&item(
                    "No visible windows",
                    0,
                    false,
                    self.mtm,
                    &self._target,
                ));
            }
            WindowsStatus::Ready(windows) => {
                for (index, window) in windows.iter().enumerate() {
                    let title = window_menu_title(&window.app_name, &window.title);
                    self.menu.addItem(&item(
                        &title,
                        TAG_FOCUS_BASE + index as isize,
                        true,
                        self.mtm,
                        &self._target,
                    ));
                }
            }
        }
        self.menu.addItem(&separator(self.mtm));
        self.menu.addItem(&item(
            if snapshot.cli_available {
                "Open Luma"
            } else {
                "Luma CLI unavailable — Set path…"
            },
            TAG_OPEN_LUMA,
            snapshot.cli_available,
            self.mtm,
            &self._target,
        ));
        self.menu.addItem(&item(
            "Open /settings in Luma",
            TAG_OPEN_SETTINGS,
            snapshot.cli_available,
            self.mtm,
            &self._target,
        ));
        self.menu
            .addItem(&item("Refresh", TAG_REFRESH, true, self.mtm, &self._target));
        self.menu.addItem(&item(
            if snapshot.launch_at_login {
                "Launch at Login ✓"
            } else {
                "Launch at Login"
            },
            TAG_LOGIN,
            true,
            self.mtm,
            &self._target,
        ));
        self.menu.addItem(&item(
            "Quit Luma Menu Bar",
            TAG_QUIT,
            true,
            self.mtm,
            &self._target,
        ));

        // Keep the brand mark stable; the menu carries the detailed Wordbook and warning state.
        self.menu.update();
    }
}

fn window_menu_title(app_name: &str, title: &str) -> String {
    if title == "Untitled" || title.trim().is_empty() {
        app_name.to_string()
    } else {
        format!("{app_name} — {title}")
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
