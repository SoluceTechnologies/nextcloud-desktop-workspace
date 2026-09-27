use objc2::runtime::{AnyClass, AnyObject, Imp, Sel};
use objc2::{msg_send, sel};
use objc2_foundation::NSPoint;
use std::sync::OnceLock;

type MouseMovedImplementation = unsafe extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject);

static ORIGINAL_MOUSE_MOVED: OnceLock<Imp> = OnceLock::new();

pub fn deliver_mouse_moves_only_to_topmost_webview() {
    let Some(observer_class) = AnyClass::get(c"WKMouseTrackingObserver") else { return };
    let Some(method) = observer_class.instance_method(sel!(mouseMoved:)) else { return };
    ORIGINAL_MOUSE_MOVED.get_or_init(|| unsafe {
        let replacement: MouseMovedImplementation = mouse_moved_when_topmost;
        method.set_implementation(std::mem::transmute::<MouseMovedImplementation, Imp>(replacement))
    });
}

unsafe extern "C-unwind" fn mouse_moved_when_topmost(observer: *mut AnyObject, selector: Sel, event: *mut AnyObject) {
    if !observer_tracks_view_under_pointer(observer, event) {
        return;
    }
    let Some(original) = ORIGINAL_MOUSE_MOVED.get() else { return };
    let original = std::mem::transmute::<Imp, MouseMovedImplementation>(*original);
    original(observer, selector, event);
}

unsafe fn observer_tracks_view_under_pointer(observer: *mut AnyObject, event: *mut AnyObject) -> bool {
    let window: *mut AnyObject = msg_send![event, window];
    if window.is_null() {
        return true;
    }
    let content_view: *mut AnyObject = msg_send![window, contentView];
    if content_view.is_null() {
        return true;
    }
    let location_in_window: NSPoint = msg_send![event, locationInWindow];
    let frame_view: *mut AnyObject = msg_send![content_view, superview];
    let point_in_frame_view: NSPoint = if frame_view.is_null() {
        location_in_window
    } else {
        msg_send![frame_view, convertPoint: location_in_window, fromView: std::ptr::null_mut::<AnyObject>()]
    };
    let mut view: *mut AnyObject = msg_send![content_view, hitTest: point_in_frame_view];
    while !view.is_null() {
        let tracking_areas: *mut AnyObject = msg_send![view, trackingAreas];
        let tracking_area_count: usize = msg_send![tracking_areas, count];
        for index in 0..tracking_area_count {
            let tracking_area: *mut AnyObject = msg_send![tracking_areas, objectAtIndex: index];
            let owner: *mut AnyObject = msg_send![tracking_area, owner];
            if owner == observer {
                return true;
            }
        }
        view = msg_send![view, superview];
    }
    false
}

#[cfg(test)]
#[path = "../tests/unit/mouse_tracking.rs"]
mod tests;
