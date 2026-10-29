use super::*;

#[test]
fn replaces_webkit_mouse_moved_with_topmost_check() {
    deliver_mouse_moves_only_to_topmost_webview();
    let observer_class = AnyClass::get(c"WKMouseTrackingObserver").expect("WebKit mouse tracking observer class");
    let method = observer_class.instance_method(sel!(mouseMoved:)).expect("mouseMoved: method");
    let replacement: MouseMovedImplementation = mouse_moved_when_topmost;
    assert_eq!(method.implementation() as usize, replacement as usize);
    assert!(ORIGINAL_MOUSE_MOVED.get().is_some_and(|original| *original as usize != replacement as usize));
}
