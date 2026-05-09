//! Seconds-since-last-user-input on macOS via `CGEventSourceSecondsSinceLastEventType`.
//! Returns 0.0 on non-macOS platforms so the rest of the scheduler logic
//! can compile and behave as if the user is always active in CI.

#[cfg(target_os = "macos")]
mod sys {
    // `kCGEventSourceStateHIDSystemState` = 1
    // `kCGAnyInputEventType` = ~0u32 (every input event type)
    const HID_SYSTEM_STATE: u32 = 1;
    const ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state_id: u32, event_type: u32) -> f64;
    }

    pub fn seconds_since_last_input() -> f64 {
        unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT_EVENT_TYPE) }
    }
}

#[cfg(target_os = "macos")]
pub fn seconds_since_last_input() -> f64 {
    sys::seconds_since_last_input()
}

#[cfg(not(target_os = "macos"))]
pub fn seconds_since_last_input() -> f64 {
    0.0
}
