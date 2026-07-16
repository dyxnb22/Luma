//! Accessibility trust + Cmd+V paste synthesis.
//! Accessibility / AXUIElement helpers. Unsafe FFI for AX APIs lives in this file with safety comments.

use async_trait::async_trait;
#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub use luma_application::{
    AccessibilityError, AccessibilityPort as Accessibility, FakeAccessibility,
};

/// Live macOS adapter using ApplicationServices + CoreGraphics.
pub struct MacAccessibility {
    /// Bumped when a paste wait is abandoned.
    ax_op_generation: Arc<AtomicU64>,
    /// Serializes the final generation check with Cmd+V synthesis.
    ax_side_effect_gate: Arc<Mutex<()>>,
}

impl Default for MacAccessibility {
    fn default() -> Self {
        Self::new()
    }
}

impl MacAccessibility {
    pub fn new() -> Self {
        Self {
            ax_op_generation: Arc::new(AtomicU64::new(0)),
            ax_side_effect_gate: Arc::new(Mutex::new(())),
        }
    }

    /// SAFETY: `AXIsProcessTrusted` is a pure query with no pointer args.
    pub fn probe_trusted() -> bool {
        #[cfg(target_os = "macos")]
        {
            unsafe { AXIsProcessTrusted() }
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    fn synthesize_cmd_v() -> Result<(), AccessibilityError> {
        #[cfg(not(target_os = "macos"))]
        {
            Err(AccessibilityError::PasteFailed(
                "macOS accessibility APIs are unavailable on this platform".into(),
            ))
        }
        #[cfg(target_os = "macos")]
        {
            // SAFETY: CGEventCreateKeyboardEvent returns a retained CF object or null.
            // We release each non-null event after posting. Flags/keycodes are constants.
            unsafe {
                let down = CGEventCreateKeyboardEvent(std::ptr::null(), KEYCODE_V, true);
                let up = CGEventCreateKeyboardEvent(std::ptr::null(), KEYCODE_V, false);
                if down.is_null() || up.is_null() {
                    if !down.is_null() {
                        CFRelease(down);
                    }
                    if !up.is_null() {
                        CFRelease(up);
                    }
                    return Err(AccessibilityError::PasteFailed(
                        "CGEventCreateKeyboardEvent returned null".into(),
                    ));
                }
                CGEventSetFlags(down, K_CG_EVENT_FLAG_MASK_COMMAND);
                CGEventSetFlags(up, K_CG_EVENT_FLAG_MASK_COMMAND);
                CGEventPost(K_CG_HID_EVENT_TAP, down);
                CGEventPost(K_CG_HID_EVENT_TAP, up);
                CFRelease(down);
                CFRelease(up);
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: *const c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

#[cfg(target_os = "macos")]
const K_CG_HID_EVENT_TAP: u32 = 0;
#[cfg(target_os = "macos")]
const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x0010_0000;
#[cfg(target_os = "macos")]
const KEYCODE_V: u16 = 9;

#[async_trait]
impl Accessibility for MacAccessibility {
    fn is_trusted(&self) -> bool {
        Self::probe_trusted()
    }

    async fn paste_clipboard(&self) -> Result<(), AccessibilityError> {
        if !self.is_trusted() {
            return Err(AccessibilityError::NotTrusted);
        }
        let generation = self.ax_op_generation.load(Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        if self.ax_op_generation.load(Ordering::SeqCst) != generation {
            return Err(AccessibilityError::PasteFailed(
                "paste abandoned after timeout".into(),
            ));
        }
        let op_generation = Arc::clone(&self.ax_op_generation);
        let side_effect_gate = Arc::clone(&self.ax_side_effect_gate);
        tokio::task::spawn_blocking(move || {
            let _gate = side_effect_gate.lock().map_err(|_| {
                AccessibilityError::PasteFailed("accessibility operation lock poisoned".into())
            })?;
            if op_generation.load(Ordering::SeqCst) != generation {
                return Err(AccessibilityError::PasteFailed(
                    "paste abandoned after timeout".into(),
                ));
            }
            Self::synthesize_cmd_v()
        })
        .await
        .map_err(|e| AccessibilityError::PasteFailed(e.to_string()))?
    }

    fn abandon_pending_ax_ops(&self) {
        self.ax_op_generation.fetch_add(1, Ordering::SeqCst);
        // If synthesis already crossed the gate, wait for the OS call to finish
        // before reporting the timeout to the caller.
        drop(self.ax_side_effect_gate.lock());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_denied_never_succeeds() {
        let ax = FakeAccessibility {
            trusted: false,
            paste_ok: true,
        };
        assert!(!ax.is_trusted());
        assert!(matches!(
            ax.paste_clipboard().await,
            Err(AccessibilityError::NotTrusted)
        ));
    }
}
