//! Menú del Dock (clic derecho en el icono): se añade `applicationDockMenu:` al
//! delegado de NSApplication que crea tao y se devuelve un menú de muda.

use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};

use objc2::runtime::{AnyObject, Imp, Sel};
use objc2::{class, msg_send, sel};

static MENU: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

extern "C-unwind" fn dock_menu(_this: *mut AnyObject, _cmd: Sel, _sender: *mut AnyObject) -> *mut AnyObject {
    MENU.load(Ordering::SeqCst).cast()
}

/// El puntero debe venir de un `muda::Menu` que se mantenga vivo.
pub fn set_menu(ns_menu: *mut c_void) {
    MENU.store(ns_menu, Ordering::SeqCst);
}

/// Llamar con el bucle de eventos ya iniciado (el delegado ya existe).
pub fn install() {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        if delegate.is_null() {
            return;
        }
        let cls = objc2::ffi::object_getClass(delegate) as *mut objc2::runtime::AnyClass;
        let imp: Imp = std::mem::transmute(
            dock_menu as extern "C-unwind" fn(*mut AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
        );
        objc2::ffi::class_addMethod(cls, sel!(applicationDockMenu:), imp, c"@@:@".as_ptr());
    }
}
