//! Visible window list + focus via CoreGraphics / Accessibility.
//! Window listing/focus via Accessibility APIs. Unsafe FFI for AX calls lives in this file only.
//! Tests must use [`luma_application::FakeWindowCatalog`], never real focus.

use async_trait::async_trait;
use luma_application::{WindowCatalogPort, WindowEntry, WindowError};
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Live macOS adapter.
pub struct MacWindowCatalog {
    previous_frontmost: Mutex<Option<String>>,
    paste_target: Mutex<Option<String>>,
    /// Bumped when a paste/focus wait is abandoned.
    ax_op_generation: Arc<AtomicU64>,
    /// Serializes the final generation check with AX focus side effects.
    ax_side_effect_gate: Arc<Mutex<()>>,
}

impl Default for MacWindowCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl MacWindowCatalog {
    pub fn new() -> Self {
        Self {
            previous_frontmost: Mutex::new(None),
            paste_target: Mutex::new(None),
            ax_op_generation: Arc::new(AtomicU64::new(0)),
            ax_side_effect_gate: Arc::new(Mutex::new(())),
        }
    }

    fn ax_op_still_valid(&self, generation: u64) -> bool {
        self.ax_op_generation.load(Ordering::SeqCst) == generation
    }

    fn set_paste_target_locked(&self, label: Option<String>) {
        if let Ok(mut g) = self.paste_target.lock() {
            *g = label;
        }
    }

    fn is_ignored_app(name: &str) -> bool {
        let lower = name.to_lowercase();
        // Localized Terminal.app name on zh-Hans macOS is 「终端」 (not "Terminal").
        if name == "终端" || name == "終端機" || name == "ターミナル" {
            return true;
        }
        matches!(
            lower.as_str(),
            "terminal"
                | "iterm"
                | "iterm2"
                | "ghostty"
                | "alacritty"
                | "kitty"
                | "wezterm"
                | "wezterm-gui"
                | "warp"
                | "hyper"
                | "tabby"
                | "rio"
                | "termius"
                | "console"
                | "luma"
                | "loginwindow"
                | "window server"
                | "dock"
                | "control center"
                | "systemuiserver"
                | "notification center"
                | "spotlight"
        ) || lower.contains("terminal")
            || lower.contains("iterm")
            || lower.contains("ghostty")
            || lower.contains("alacritty")
    }

    fn list_windows_blocking() -> Result<Vec<WindowEntry>, WindowError> {
        // SAFETY: CGWindowListCopyWindowInfo returns a retained CFArray or null.
        let info = unsafe {
            CGWindowListCopyWindowInfo(
                K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
                K_CG_NULL_WINDOW_ID,
            )
        };
        if info.is_null() {
            return Err(WindowError::Unavailable(
                "CGWindowListCopyWindowInfo returned null".into(),
            ));
        }
        let mut out = Vec::new();
        let count = unsafe { CFArrayGetCount(info) };
        for i in 0..count {
            let item = unsafe { CFArrayGetValueAtIndex(info, i) };
            if item.is_null() {
                continue;
            }
            let dict = item as CFDictionaryRef;
            let layer = cf_dict_i64(dict, "kCGWindowLayer").unwrap_or(0);
            if layer != 0 {
                continue;
            }
            let owner_pid = cf_dict_i64(dict, "kCGWindowOwnerPID").unwrap_or(0) as u32;
            if owner_pid == 0 {
                continue;
            }
            let app_name = cf_dict_string(dict, "kCGWindowOwnerName").unwrap_or_default();
            if app_name.is_empty() || Self::is_ignored_app(&app_name) {
                continue;
            }
            let window_number = cf_dict_i64(dict, "kCGWindowNumber").unwrap_or(0);
            let title = cf_dict_string(dict, "kCGWindowName").unwrap_or_default();
            let display_title = if title.trim().is_empty() {
                "Untitled".to_string()
            } else {
                title
            };
            let is_on_screen = cf_dict_bool(dict, "kCGWindowIsOnscreen").unwrap_or(true);
            out.push(WindowEntry {
                id: format!("pid:{owner_pid}|num:{window_number}"),
                app_name,
                app_bundle_id: None,
                title: display_title,
                is_on_screen,
                layer,
                owner_pid,
            });
        }
        unsafe { CFRelease(info) };
        Ok(out)
    }

    fn snapshot_blocking(entries: &[WindowEntry]) -> Option<String> {
        entries
            .iter()
            .find(|e| e.is_on_screen && !Self::is_ignored_app(&e.app_name))
            .map(|e| e.app_name.clone())
    }

    fn focus_app_blocking(app_name: &str) -> Result<(), WindowError> {
        let entries = Self::list_windows_blocking()?;
        let Some(target) = entries
            .iter()
            .find(|e| e.app_name == app_name && e.is_on_screen)
        else {
            return Err(WindowError::NotFound(format!("app {app_name}")));
        };
        Self::focus_blocking(&target.id)
    }

    fn frontmost_app_blocking() -> Result<Option<String>, WindowError> {
        let entries = Self::list_windows_blocking()?;
        Ok(Self::snapshot_blocking(&entries))
    }

    fn focus_blocking(id: &str) -> Result<(), WindowError> {
        if !unsafe { AXIsProcessTrusted() } {
            return Err(WindowError::PermissionRequired {
                capability: "accessibility".into(),
                guidance: "Grant Accessibility to the app that launched Luma in System Settings → Privacy & Security → Accessibility, then retry.".into(),
            });
        }
        let (pid, _) = parse_window_id(id)
            .ok_or_else(|| WindowError::Unavailable(format!("invalid window id: {id}")))?;
        let entries = Self::list_windows_blocking()?;
        let Some(target) = entries.iter().find(|e| e.id == id) else {
            return Err(WindowError::NotFound(format!("window {id}")));
        };
        let app = unsafe { AXUIElementCreateApplication(pid as i32) };
        if app.is_null() {
            return Err(WindowError::Unavailable(
                "AXUIElementCreateApplication failed".into(),
            ));
        }
        let result = raise_window(app, target);
        unsafe { CFRelease(app as *const c_void) };
        result
    }

    /// Sync snapshot for composition root (before async runtime is attached).
    pub fn snapshot_previous_frontmost_app_sync(&self) -> Result<Option<String>, WindowError> {
        let entries = Self::list_windows_blocking()?;
        let label = Self::snapshot_blocking(&entries);
        *self
            .previous_frontmost
            .lock()
            .map_err(|_| WindowError::Unavailable("previous_frontmost lock poisoned".into()))? =
            label.clone();
        self.set_paste_target_locked(label.clone());
        Ok(label)
    }
}

fn parse_window_id(id: &str) -> Option<(u32, i64)> {
    let rest = id.strip_prefix("pid:")?;
    let (pid_s, num_s) = rest.split_once("|num:")?;
    Some((pid_s.parse().ok()?, num_s.parse().ok()?))
}

fn raise_window(app: AXUIElementRef, target: &WindowEntry) -> Result<(), WindowError> {
    let frontmost = ax_frontmost()?;
    let windows_attr = ax_windows()?;
    let title_attr = ax_title()?;
    let raise_attr = ax_raise()?;

    let set_front =
        unsafe { AXUIElementSetAttributeValue(app, frontmost, kCFBooleanTrue as CFTypeRef) };
    if set_front != 0 {
        return Err(map_ax_error(
            set_front,
            "could not bring application to front",
        ));
    }

    let mut windows_ref: CFTypeRef = ptr::null_mut();
    let copy = unsafe { AXUIElementCopyAttributeValue(app, windows_attr, &mut windows_ref) };
    if copy != 0 || windows_ref.is_null() {
        return Err(map_ax_error(
            if copy != 0 { copy } else { K_AX_ERROR_FAILURE },
            "could not read application windows (AXWindows)",
        ));
    }
    let windows = windows_ref as CFArrayRef;
    let count = unsafe { CFArrayGetCount(windows) };

    // Collect AX windows with titles for matching.
    let mut ax_wins: Vec<(AXUIElementRef, String)> = Vec::new();
    for i in 0..count {
        let win = unsafe { CFArrayGetValueAtIndex(windows, i) } as AXUIElementRef;
        if win.is_null() {
            continue;
        }
        let mut title_ref: CFTypeRef = ptr::null_mut();
        let ok = unsafe { AXUIElementCopyAttributeValue(win, title_attr, &mut title_ref) };
        let title = if ok == 0 && !title_ref.is_null() {
            let s = cf_string_to_rust(title_ref as CFStringRef);
            unsafe { CFRelease(title_ref as *const c_void) };
            s.unwrap_or_default()
        } else {
            String::new()
        };
        ax_wins.push((win, title));
    }

    let want_title = if target.title == "Untitled" {
        String::new()
    } else {
        target.title.clone()
    };

    // Same-title siblings: disambiguate by ordinal among CG windows of this pid+title.
    let cg_siblings: Vec<WindowEntry> = match MacWindowCatalog::list_windows_blocking() {
        Ok(list) => list
            .into_iter()
            .filter(|e| {
                e.owner_pid == target.owner_pid
                    && if want_title.is_empty() {
                        e.title == "Untitled"
                    } else {
                        e.title == want_title
                    }
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    let ordinal = cg_siblings
        .iter()
        .position(|e| e.id == target.id)
        .unwrap_or(0);

    let mut title_matches: Vec<AXUIElementRef> = ax_wins
        .iter()
        .filter(|(_, t)| {
            if want_title.is_empty() {
                t.is_empty()
            } else {
                t == &want_title
            }
        })
        .map(|(w, _)| *w)
        .collect();

    let chosen = if title_matches.is_empty() {
        None
    } else if title_matches.len() == 1 {
        Some(title_matches.remove(0))
    } else {
        Some(title_matches[ordinal.min(title_matches.len() - 1)])
    };

    let Some(win) = chosen else {
        unsafe { CFRelease(windows_ref as *const c_void) };
        return Err(WindowError::Unavailable(format!(
            "could not match window {:?} in app — try `win` to re-search",
            target.title
        )));
    };

    let raise = unsafe { AXUIElementPerformAction(win, raise_attr) };
    unsafe { CFRelease(windows_ref as *const c_void) };
    if raise != 0 {
        return Err(map_ax_error(raise, "AXRaise failed"));
    }
    Ok(())
}

/// Map AXError codes. Only `kAXErrorAPIDisabled` is treated as permission.
fn map_ax_error(code: AXError, context: &str) -> WindowError {
    const K_AX_ERROR_API_DISABLED: AXError = -25211;
    if code == K_AX_ERROR_API_DISABLED || !unsafe { AXIsProcessTrusted() } {
        return WindowError::PermissionRequired {
            capability: "accessibility".into(),
            guidance: "Grant Accessibility to the app that launched Luma in System Settings → Privacy & Security → Accessibility, then retry.".into(),
        };
    }
    WindowError::Unavailable(format!("{context} (AXError {code})"))
}

const K_AX_ERROR_FAILURE: AXError = -25200;

#[async_trait]
impl WindowCatalogPort for MacWindowCatalog {
    async fn snapshot_previous_frontmost_app(&self) -> Result<Option<String>, WindowError> {
        let label = tokio::task::spawn_blocking(|| {
            let entries = MacWindowCatalog::list_windows_blocking()?;
            Ok::<_, WindowError>(MacWindowCatalog::snapshot_blocking(&entries))
        })
        .await
        .map_err(|e| WindowError::Unavailable(e.to_string()))??;
        *self
            .previous_frontmost
            .lock()
            .map_err(|_| WindowError::Unavailable("previous_frontmost lock poisoned".into()))? =
            label.clone();
        self.set_paste_target_locked(label.clone());
        Ok(label)
    }

    async fn previous_frontmost_app(&self) -> Option<String> {
        self.previous_frontmost.lock().ok().and_then(|g| g.clone())
    }

    async fn paste_target_app(&self) -> Option<String> {
        self.paste_target.lock().ok().and_then(|g| g.clone())
    }

    async fn set_paste_target_app(&self, app_name: Option<String>) {
        self.set_paste_target_locked(app_name);
    }

    async fn focus_app_by_name(&self, app_name: &str) -> Result<(), WindowError> {
        let generation = self.ax_op_generation.load(Ordering::SeqCst);
        let name = app_name.to_string();
        let op_generation = Arc::clone(&self.ax_op_generation);
        let side_effect_gate = Arc::clone(&self.ax_side_effect_gate);
        tokio::task::spawn_blocking(move || {
            let _gate = side_effect_gate
                .lock()
                .map_err(|_| WindowError::Unavailable("window operation lock poisoned".into()))?;
            if op_generation.load(Ordering::SeqCst) != generation {
                return Err(WindowError::Unavailable(
                    "paste focus abandoned after timeout".into(),
                ));
            }
            MacWindowCatalog::focus_app_blocking(&name)
        })
        .await
        .map_err(|e| WindowError::Unavailable(e.to_string()))??;
        if !self.ax_op_still_valid(generation) {
            return Err(WindowError::Unavailable(
                "paste focus abandoned after timeout".into(),
            ));
        }
        self.set_paste_target_locked(Some(app_name.to_string()));
        Ok(())
    }

    async fn frontmost_app_name(&self) -> Result<Option<String>, WindowError> {
        tokio::task::spawn_blocking(MacWindowCatalog::frontmost_app_blocking)
            .await
            .map_err(|e| WindowError::Unavailable(e.to_string()))?
    }

    async fn list_windows(&self) -> Result<Vec<WindowEntry>, WindowError> {
        tokio::task::spawn_blocking(MacWindowCatalog::list_windows_blocking)
            .await
            .map_err(|e| WindowError::Unavailable(e.to_string()))?
    }

    async fn focus(&self, id: &str) -> Result<(), WindowError> {
        let generation = self.ax_op_generation.load(Ordering::SeqCst);
        let id = id.to_string();
        let app_name = {
            let id_for_lookup = id.clone();
            tokio::task::spawn_blocking(move || {
                let entries = MacWindowCatalog::list_windows_blocking()?;
                Ok::<_, WindowError>(
                    entries
                        .into_iter()
                        .find(|e| e.id == id_for_lookup)
                        .map(|e| e.app_name),
                )
            })
            .await
            .map_err(|e| WindowError::Unavailable(e.to_string()))??
        };
        let op_generation = Arc::clone(&self.ax_op_generation);
        let side_effect_gate = Arc::clone(&self.ax_side_effect_gate);
        tokio::task::spawn_blocking(move || {
            let _gate = side_effect_gate
                .lock()
                .map_err(|_| WindowError::Unavailable("window operation lock poisoned".into()))?;
            if op_generation.load(Ordering::SeqCst) != generation {
                return Err(WindowError::Unavailable(
                    "window focus abandoned after timeout".into(),
                ));
            }
            MacWindowCatalog::focus_blocking(&id)
        })
        .await
        .map_err(|e| WindowError::Unavailable(e.to_string()))??;
        if !self.ax_op_still_valid(generation) {
            return Err(WindowError::Unavailable(
                "window focus abandoned after timeout".into(),
            ));
        }
        if let Some(name) = app_name {
            self.set_paste_target_locked(Some(name));
        }
        Ok(())
    }

    fn abandon_pending_ax_ops(&self) {
        self.ax_op_generation.fetch_add(1, Ordering::SeqCst);
        // If focus already crossed the gate, wait for the OS call to finish
        // before reporting the timeout to the caller.
        drop(self.ax_side_effect_gate.lock());
    }
}

// --- FFI ---

type CFIndex = isize;
type CFTypeRef = *mut c_void;
type CFArrayRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFStringRef = *const c_void;
type AXUIElementRef = *mut c_void;
type AXError = i32;

const K_CG_NULL_WINDOW_ID: u32 = 0;
const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1 << 0;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_CF_NUMBER_SINT64_TYPE: i32 = 4;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFArrayRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFArrayGetCount(the_array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(the_array: CFArrayRef, idx: CFIndex) -> *const c_void;
    fn CFDictionaryGetValue(the_dict: CFDictionaryRef, key: *const c_void) -> *const c_void;
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const i8,
        encoding: u32,
    ) -> CFStringRef;
    fn CFStringGetTypeID() -> usize;
    fn CFGetTypeID(cf: *const c_void) -> usize;
    fn CFStringGetCStringPtr(the_string: CFStringRef, encoding: u32) -> *const i8;
    fn CFStringGetLength(the_string: CFStringRef) -> CFIndex;
    fn CFStringGetMaximumSizeForEncoding(length: CFIndex, encoding: u32) -> CFIndex;
    fn CFStringGetCString(
        the_string: CFStringRef,
        buffer: *mut i8,
        buffer_size: CFIndex,
        encoding: u32,
    ) -> bool;
    fn CFNumberGetValue(number: *const c_void, the_type: i32, value_ptr: *mut c_void) -> bool;
    fn CFBooleanGetValue(boolean: *const c_void) -> bool;
    static kCFBooleanTrue: *const c_void;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
}

fn intern_cf_str(key: &'static str) -> Result<CFStringRef, WindowError> {
    static CACHE: OnceLock<Mutex<Vec<(String, usize)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = match cache.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some((_, ptr)) = guard.iter().find(|(k, _)| k == key) {
        return Ok(*ptr as CFStringRef);
    }
    // Keys are static ASCII; CString::new only fails on interior NUL.
    let c = CString::new(key)
        .map_err(|_| WindowError::Unavailable(format!("CFString key contains NUL: {key}")))?;
    let s =
        unsafe { CFStringCreateWithCString(ptr::null(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8) };
    if s.is_null() {
        return Err(WindowError::Unavailable(format!(
            "CFStringCreateWithCString failed for {key}"
        )));
    }
    guard.push((key.to_string(), s as usize));
    Ok(s)
}

fn ax_windows() -> Result<CFStringRef, WindowError> {
    intern_cf_str("AXWindows")
}
fn ax_title() -> Result<CFStringRef, WindowError> {
    intern_cf_str("AXTitle")
}
fn ax_frontmost() -> Result<CFStringRef, WindowError> {
    intern_cf_str("AXFrontmost")
}
fn ax_raise() -> Result<CFStringRef, WindowError> {
    intern_cf_str("AXRaise")
}

fn with_cf_key<T>(key: &str, f: impl FnOnce(CFStringRef) -> T) -> Option<T> {
    // Dictionary keys are static ASCII; interior NUL is treated as a miss.
    let c = CString::new(key).ok()?;
    let s =
        unsafe { CFStringCreateWithCString(ptr::null(), c.as_ptr(), K_CF_STRING_ENCODING_UTF8) };
    if s.is_null() {
        return None;
    }
    let out = f(s);
    unsafe { CFRelease(s) };
    Some(out)
}

fn cf_dict_string(dict: CFDictionaryRef, key: &str) -> Option<String> {
    with_cf_key(key, |k| {
        let v = unsafe { CFDictionaryGetValue(dict, k) };
        if v.is_null() {
            return None;
        }
        cf_string_to_rust(v as CFStringRef)
    })
    .flatten()
}

fn cf_dict_i64(dict: CFDictionaryRef, key: &str) -> Option<i64> {
    with_cf_key(key, |k| {
        let v = unsafe { CFDictionaryGetValue(dict, k) };
        if v.is_null() {
            return None;
        }
        let mut n: i64 = 0;
        let ok =
            unsafe { CFNumberGetValue(v, K_CF_NUMBER_SINT64_TYPE, (&mut n as *mut i64).cast()) };
        ok.then_some(n)
    })
    .flatten()
}

fn cf_dict_bool(dict: CFDictionaryRef, key: &str) -> Option<bool> {
    with_cf_key(key, |k| {
        let v = unsafe { CFDictionaryGetValue(dict, k) };
        if v.is_null() {
            return None;
        }
        Some(unsafe { CFBooleanGetValue(v) })
    })
    .flatten()
}

fn cf_string_to_rust(s: CFStringRef) -> Option<String> {
    if s.is_null() {
        return None;
    }
    if unsafe { CFGetTypeID(s) } != unsafe { CFStringGetTypeID() } {
        return None;
    }
    let ptr = unsafe { CFStringGetCStringPtr(s, K_CF_STRING_ENCODING_UTF8) };
    if !ptr.is_null() {
        let c = unsafe { CStr::from_ptr(ptr) };
        return Some(c.to_string_lossy().into_owned());
    }
    let len = unsafe { CFStringGetLength(s) };
    let max = unsafe { CFStringGetMaximumSizeForEncoding(len, K_CF_STRING_ENCODING_UTF8) } + 1;
    if max <= 0 {
        return Some(String::new());
    }
    let mut buf = vec![0i8; max as usize];
    let ok = unsafe { CFStringGetCString(s, buf.as_mut_ptr(), max, K_CF_STRING_ENCODING_UTF8) };
    if !ok {
        return None;
    }
    let c = unsafe { CStr::from_ptr(buf.as_ptr()) };
    Some(c.to_string_lossy().into_owned())
}
