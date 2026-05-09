//! macOS Accessibility API helpers — read the focused window's title and
//! the active document's URL (for browsers) so the cat sees what app *and
//! what inside that app* is in focus. Patterned on tambourine-voice's
//! `active_app_context/macos.rs`.
//!
//! Requires the user to grant Accessibility permission to the app via
//! System Settings → Privacy & Security → Accessibility. Without that, all
//! reads return `None` and we fall back to the bundle-id-only signal.

#![cfg(target_os = "macos")]
#![allow(dead_code)]

use std::ffi::c_void;
use std::ptr;

use core_foundation::base::{CFType, CFTypeID, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};

type AxUiElementRef = *const c_void;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXUIElementGetTypeID() -> CFTypeID;
    fn AXUIElementCreateApplication(process_identifier: i32) -> AxUiElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AxUiElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
}

const AX_SUCCESS: i32 = 0;

/// Whether macOS has granted Accessibility permission to this app.
/// First call from a non-trusted process triggers the system prompt.
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

fn create_application_element(pid: i32) -> Option<CFType> {
    let raw = unsafe { AXUIElementCreateApplication(pid) } as CFTypeRef;
    if raw.is_null() {
        None
    } else {
        Some(unsafe { CFType::wrap_under_create_rule(raw) })
    }
}

fn copy_attribute_value(element: &CFType, attribute: &str) -> Option<CFType> {
    let element_type_id = unsafe { AXUIElementGetTypeID() };
    if element.type_of() != element_type_id {
        return None;
    }
    let attribute_name = CFString::new(attribute);
    let mut raw: CFTypeRef = ptr::null();
    let status = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef() as AxUiElementRef,
            attribute_name.as_concrete_TypeRef(),
            &raw mut raw,
        )
    };
    if status != AX_SUCCESS || raw.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(raw) })
}

fn copy_element_attribute(element: &CFType, attribute: &str) -> Option<CFType> {
    let value = copy_attribute_value(element, attribute)?;
    let element_type_id = unsafe { AXUIElementGetTypeID() };
    if value.type_of() == element_type_id {
        Some(value)
    } else {
        None
    }
}

fn copy_string_attribute(element: &CFType, attribute: &str) -> Option<String> {
    let value = copy_attribute_value(element, attribute)?;
    let cf_string = value.downcast::<CFString>()?;
    let owned = cf_string.to_string();
    let trimmed = owned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn focused_window(app_element: &CFType) -> Option<CFType> {
    copy_element_attribute(app_element, "AXFocusedWindow")
        .or_else(|| copy_element_attribute(app_element, "AXFocusedUIElement"))
}

/// Read the focused window's title and (if the app is a browser) the
/// active tab's document URL. Returns `(title, url)` — either or both may
/// be `None` if Accessibility access wasn't granted or the app doesn't
/// expose those attributes.
pub fn focused_window_details(pid: i32) -> (Option<String>, Option<String>) {
    if !is_trusted() {
        return (None, None);
    }
    let Some(app_element) = create_application_element(pid) else {
        return (None, None);
    };
    let Some(window_element) = focused_window(&app_element) else {
        return (None, None);
    };
    let title = copy_string_attribute(&window_element, "AXTitle");
    let url = copy_string_attribute(&window_element, "AXDocument");
    (title, url)
}
