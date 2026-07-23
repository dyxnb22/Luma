use crate::model::{
    ActionStatus, LoginItemState, MenuAction, MenuSnapshot, SharedMenuSnapshot, WindowsStatus,
    WordbookStatus,
};
use dispatch2::DispatchQueue;
use luma_platform_macos::MacAccessibility;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, ProtocolObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSImage, NSLineCapStyle, NSMenu, NSMenuDelegate, NSMenuItem,
    NSStatusBar, NSStatusItem,
};
use objc2_foundation::{ns_string, NSObjectProtocol, NSPoint, NSSize, NSString};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const TAG_REVIEW_DUE: isize = 10;
const TAG_OPEN_LUMA: isize = 11;
const TAG_OPEN_SETTINGS: isize = 12;
const TAG_REFRESH: isize = 13;
const TAG_LOGIN: isize = 14;
const TAG_QUIT: isize = 15;
const MAX_MENU_TITLE_CHARS: usize = 96;
const SCREEN_RECORDING_HINT: &str =
    "Some window titles unavailable — grant Screen Recording to Luma Menu Bar.app";
const ACCESSIBILITY_FOCUS_HINT: &str = "Focus requires Accessibility for Luma Menu Bar.app";

static ACTIONS: OnceLock<Mutex<Option<Sender<MenuAction>>>> = OnceLock::new();

struct SnapshotNotifierState {
    target: AtomicPtr<MenuTarget>,
    queued: AtomicBool,
}

/// Bridges worker publications back to AppKit without polling from the UI thread.
#[derive(Clone)]
pub struct SnapshotNotifier {
    state: Arc<SnapshotNotifierState>,
}

impl SnapshotNotifier {
    fn new(target: &MenuTarget) -> Self {
        Self {
            state: Arc::new(SnapshotNotifierState {
                target: AtomicPtr::new(target as *const MenuTarget as *mut MenuTarget),
                queued: AtomicBool::new(false),
            }),
        }
    }

    /// Queue at most one main-thread redraw for the latest published snapshot.
    pub fn schedule(&self) {
        if self.state.target.load(Ordering::Acquire).is_null()
            || self.state.queued.swap(true, Ordering::AcqRel)
        {
            return;
        }
        let state = Arc::clone(&self.state);
        DispatchQueue::main().exec_async(move || {
            state.queued.store(false, Ordering::Release);
            let target = state.target.load(Ordering::Acquire);
            if target.is_null() {
                return;
            }
            // SAFETY: the target is created and destroyed on AppKit's main thread. Drop clears
            // this pointer before releasing the target, and this block also runs on the main
            // queue, so it cannot dereference a target after that clear.
            unsafe { (&*target).apply_latest_snapshot() };
        });
    }

    fn clear(&self) {
        self.state.target.store(ptr::null_mut(), Ordering::Release);
    }
}

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
            let Some(action) = menu_action_for_tag(sender.tag()) else {
                return;
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
    snapshot_notifier: SnapshotNotifier,
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
        let snapshot_notifier = SnapshotNotifier::new(&target);
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
            snapshot_notifier,
        };
        controller.apply_snapshot(initial_snapshot);
        controller
    }

    #[allow(deprecated)]
    pub fn apply_snapshot(&mut self, snapshot: MenuSnapshot) {
        render_menu(&self.menu, self.mtm, &self._target, &snapshot);
    }

    pub fn snapshot_notifier(&self) -> SnapshotNotifier {
        self.snapshot_notifier.clone()
    }
}

impl Drop for MenuController {
    fn drop(&mut self) {
        self.snapshot_notifier.clear();
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
    menu.addItem(&item(
        &snapshot_freshness_title(snapshot.captured_at_unix, unix_now()),
        0,
        false,
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
            add_window_rows(menu, windows, mtm, target);
        }
        WindowsStatus::Stale { windows, reason } => {
            menu.addItem(&item(
                &format!("Windows stale: {reason}"),
                0,
                false,
                mtm,
                target,
            ));
            add_window_rows(menu, windows, mtm, target);
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

fn menu_action_for_tag(tag: isize) -> Option<MenuAction> {
    match tag {
        TAG_REVIEW_DUE => Some(MenuAction::ReviewDue),
        TAG_OPEN_LUMA => Some(MenuAction::OpenLuma),
        TAG_OPEN_SETTINGS => Some(MenuAction::OpenSettings),
        TAG_REFRESH => Some(MenuAction::Refresh),
        TAG_LOGIN => Some(MenuAction::ToggleLaunchAtLogin),
        TAG_QUIT => Some(MenuAction::Quit),
        _ => None,
    }
}

fn windows_need_screen_recording_hint(windows: &[luma_application::WindowEntry]) -> bool {
    windows.iter().any(|window| window.title == "Untitled")
}

fn add_window_rows(
    menu: &NSMenu,
    windows: &[luma_application::WindowEntry],
    mtm: MainThreadMarker,
    target: &MenuTarget,
) {
    // This is a pure TCC query, not window or storage I/O; keeping the list visible without
    // Accessibility is intentional, but users should see the focus remediation before clicking.
    if !MacAccessibility::probe_trusted() {
        menu.addItem(&item(ACCESSIBILITY_FOCUS_HINT, 0, false, mtm, target));
    }
    if windows_need_screen_recording_hint(windows) {
        menu.addItem(&item(SCREEN_RECORDING_HINT, 0, false, mtm, target));
    }
    for window in windows {
        menu.addItem(&window_item(window, mtm, target));
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
    let title = NSString::from_str(&truncate_menu_text(title));
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

fn truncate_menu_text(text: &str) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(MAX_MENU_TITLE_CHARS).collect();
    if chars.next().is_some() {
        let prefix: String = text
            .chars()
            .take(MAX_MENU_TITLE_CHARS.saturating_sub(1))
            .collect();
        format!("{prefix}…")
    } else {
        truncated
    }
}

fn snapshot_freshness_title(captured_at_unix: u64, now_unix: u64) -> String {
    if captured_at_unix == 0 {
        return "Snapshot not refreshed yet".into();
    }
    let age = now_unix.saturating_sub(captured_at_unix);
    let label = match age {
        0..=59 => "just now".to_string(),
        60..=3_599 => format!("{}m ago", age / 60),
        3_600..=86_399 => format!("{}h ago", age / 3_600),
        _ => format!("{}d ago", age / 86_400),
    };
    format!("Snapshot updated {label}")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        login_item_title, menu_action_for_tag, snapshot_freshness_title, truncate_menu_text,
        window_menu_title, windows_need_screen_recording_hint, TAG_LOGIN, TAG_OPEN_LUMA,
        TAG_OPEN_SETTINGS, TAG_QUIT, TAG_REFRESH, TAG_REVIEW_DUE,
    };
    use crate::model::{LoginItemState, MenuAction};
    use luma_application::WindowEntry;

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

    #[test]
    fn menu_text_is_bounded_and_keeps_an_ellipsis() {
        let text = truncate_menu_text(&"a".repeat(200));
        assert_eq!(text.chars().count(), 96);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn freshness_shows_age_in_human_scale() {
        assert_eq!(
            snapshot_freshness_title(0, 100),
            "Snapshot not refreshed yet"
        );
        assert_eq!(
            snapshot_freshness_title(100, 101),
            "Snapshot updated just now"
        );
        assert_eq!(
            snapshot_freshness_title(100, 700),
            "Snapshot updated 10m ago"
        );
        assert_eq!(
            snapshot_freshness_title(100, 3_700),
            "Snapshot updated 1h ago"
        );
    }

    #[test]
    fn menu_tags_map_to_their_declared_actions() {
        assert_eq!(
            menu_action_for_tag(TAG_REVIEW_DUE),
            Some(MenuAction::ReviewDue)
        );
        assert_eq!(
            menu_action_for_tag(TAG_OPEN_LUMA),
            Some(MenuAction::OpenLuma)
        );
        assert_eq!(
            menu_action_for_tag(TAG_OPEN_SETTINGS),
            Some(MenuAction::OpenSettings)
        );
        assert_eq!(menu_action_for_tag(TAG_REFRESH), Some(MenuAction::Refresh));
        assert_eq!(
            menu_action_for_tag(TAG_LOGIN),
            Some(MenuAction::ToggleLaunchAtLogin)
        );
        assert_eq!(menu_action_for_tag(TAG_QUIT), Some(MenuAction::Quit));
        assert_eq!(menu_action_for_tag(999), None);
    }

    #[test]
    fn login_item_states_remain_distinguishable() {
        assert_eq!(
            login_item_title(&LoginItemState::NotRegistered),
            "Launch at Login"
        );
        assert!(login_item_title(&LoginItemState::Enabled).contains('✓'));
        assert!(login_item_title(&LoginItemState::RequiresApproval).contains("Approval"));
        assert!(login_item_title(&LoginItemState::NotFound).contains("unavailable"));
        assert!(login_item_title(&LoginItemState::Unavailable("error".into())).contains("error"));
    }

    #[test]
    fn missing_titles_request_screen_recording_guidance() {
        let entry = WindowEntry {
            id: "window".into(),
            app_name: "Safari".into(),
            app_bundle_id: None,
            title: "Untitled".into(),
            is_on_screen: true,
            layer: 0,
            owner_pid: 1,
        };
        assert!(windows_need_screen_recording_hint(&[entry]));
    }
}
