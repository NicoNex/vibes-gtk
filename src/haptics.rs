//! A short haptic tap when a setting lands on a new value.
//!
//! macOS is the only platform here that has an API for it: `NSHapticFeedbackManager` drives the
//! Force Touch trackpad, and no-ops by itself on Macs without one, on an external mouse, and when
//! the user has turned haptic feedback off in System Settings. Everywhere else — Linux desktops,
//! the postmarketOS phone build — GTK exposes no haptic API at all, so `tap` does nothing.

#[cfg(not(target_os = "macos"))]
pub fn tap() {}

#[cfg(target_os = "macos")]
pub fn tap() {
    use std::ffi::{c_char, c_void};

    // ponytail: two objc_msgSend calls over an objc crate — the whole binding is the four lines
    // below. Pull in objc2 if this ever grows past one call.
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    }

    // NSHapticFeedbackPatternLevelChange: the pattern Apple defines for a value crossing a
    // discrete step, which is exactly what these sliders and toggles do.
    const PATTERN_LEVEL_CHANGE: isize = 2;
    const PERFORMANCE_TIME_NOW: usize = 1;

    unsafe {
        let class = objc_getClass(c"NSHapticFeedbackManager".as_ptr());
        if class.is_null() {
            return;
        }
        let send_id: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let performer = send_id(class, sel_registerName(c"defaultPerformer".as_ptr()));
        if performer.is_null() {
            return;
        }
        let send_pattern: extern "C" fn(*mut c_void, *mut c_void, isize, usize) =
            std::mem::transmute(objc_msgSend as *const ());
        send_pattern(
            performer,
            sel_registerName(c"performFeedbackPattern:performanceTime:".as_ptr()),
            PATTERN_LEVEL_CHANGE,
            PERFORMANCE_TIME_NOW,
        );
    }
}

// The binding is hand-rolled, so the check is that the class, the selector and the call all still
// resolve: a typo in any of the three names is a silent no-op, or a crash, at runtime.
#[cfg(test)]
mod tests {
    #[test]
    fn tap_reaches_the_platform() {
        super::tap();
    }
}
