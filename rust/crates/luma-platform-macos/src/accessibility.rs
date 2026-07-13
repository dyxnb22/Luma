//! Accessibility trust + Cmd+V paste synthesis.
//! Unsafe FFI is confined here with safety comments.

use async_trait::async_trait;
use std::ffi::c_void;

pub use luma_application::{
    AccessibilityError, AccessibilityPort as Accessibility, FakeAccessibility,
};

/// Live macOS adapter using ApplicationServices + CoreGraphics.
pub struct MacAccessibility;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

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

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

const K_CG_HID_EVENT_TAP: u32 = 0;
const K_CG_EVENT_FLAG_MASK_COMMAND: u64 = 0x0010_0000;
const KEYCODE_V: u16 = 9;

impl MacAccessibility {
    /// SAFETY: `AXIsProcessTrusted` is a pure query with no pointer args.
    pub fn probe_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    fn synthesize_cmd_v() -> Result<(), AccessibilityError> {
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

#[async_trait]
impl Accessibility for MacAccessibility {
    fn is_trusted(&self) -> bool {
        Self::probe_trusted()
    }

    async fn paste_clipboard(&self) -> Result<(), AccessibilityError> {
        if !self.is_trusted() {
            return Err(AccessibilityError::NotTrusted);
        }
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        tokio::task::spawn_blocking(Self::synthesize_cmd_v)
            .await
            .map_err(|e| AccessibilityError::PasteFailed(e.to_string()))?
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
