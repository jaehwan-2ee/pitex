//! App-local IBus compatibility. Signal messages are owned while the legacy
//! synchronous key call runs, then delivered on its calling GTK thread.
use ::gio::ffi as gio;
use ::glib::{ffi as glib, gobject_ffi as object};
use std::{
    ffi::{CStr, CString},
    ptr,
    sync::{Arc, Mutex, OnceLock},
};
type SyncKey = unsafe extern "C" fn(*mut gio::GDBusProxy, u32, u32, u32) -> glib::gboolean;
struct Original {
    call: SyncKey,
    modern: bool,
}
static ORIGINAL: OnceLock<Option<Original>> = OnceLock::new();
pub fn initialize() {
    std::env::set_var("IBUS_ENABLE_SYNC_MODE", "1");
}
struct Pending {
    path: CString,
    sender: Option<CString>,
    messages: Mutex<Option<Vec<usize>>>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(messages) = self
            .messages
            .get_mut()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            for message in messages {
                unsafe {
                    object::g_object_unref(message as *mut object::GObject);
                }
            }
        }
    }
}
unsafe extern "C" fn release(data: glib::gpointer) {
    drop(Arc::from_raw(data as *const Pending));
}
unsafe fn matches_string(value: *const libc::c_char, expected: &CStr) -> bool {
    !value.is_null() && CStr::from_ptr(value) == expected
}
unsafe extern "C" fn collect(
    _connection: *mut gio::GDBusConnection,
    message: *mut gio::GDBusMessage,
    incoming: glib::gboolean,
    data: glib::gpointer,
) -> *mut gio::GDBusMessage {
    let pending = &*(data as *const Pending);
    if incoming == 0
        || gio::g_dbus_message_get_message_type(message) != gio::G_DBUS_MESSAGE_TYPE_SIGNAL
        || !matches_string(gio::g_dbus_message_get_path(message), &pending.path)
        || !matches_string(
            gio::g_dbus_message_get_interface(message),
            CStr::from_bytes_with_nul_unchecked(b"org.freedesktop.IBus.InputContext\0"),
        )
        || pending
            .sender
            .as_ref()
            .is_some_and(|sender| !matches_string(gio::g_dbus_message_get_sender(message), sender))
    {
        return message;
    }
    let mut messages = pending.messages.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(messages) = messages.as_mut() {
        messages.push(message as usize);
        ptr::null_mut()
    } else {
        message
    }
}
fn original() -> Option<&'static Original> {
    ORIGINAL
        .get_or_init(|| unsafe {
            // Keep the library loaded for the lifetime of the exported callback.
            let handle = libc::dlopen(
                b"libibus-1.0.so.5\0".as_ptr().cast(),
                libc::RTLD_LAZY | libc::RTLD_LOCAL,
            );
            if handle.is_null() {
                return None;
            }
            let symbol = libc::dlsym(
                handle,
                b"ibus_input_context_process_key_event\0".as_ptr().cast(),
            );
            if symbol.is_null() {
                return None;
            }
            Some(Original {
                call: std::mem::transmute::<*mut libc::c_void, SyncKey>(symbol),
                modern: !libc::dlsym(
                    handle,
                    b"ibus_input_context_post_process_key_event\0"
                        .as_ptr()
                        .cast(),
                )
                .is_null(),
            })
        })
        .as_ref()
}
fn forward_matches(key:u32,code:u32,modifiers:u32,original_key:u32,original_code:u32,original_state:u32)->bool {
    // IBus processing flags are not keyboard modifiers (ibus-ibustypes API).
    const KEYBOARD_STATE:u32=0x5c001fff;
    key==original_key && (code==original_code || code==0)
        && modifiers & KEYBOARD_STATE == original_state & KEYBOARD_STATE
}
#[no_mangle]
pub unsafe extern "C" fn ibus_input_context_process_key_event(
    context: *mut gio::GDBusProxy,
    keyval: u32,
    keycode: u32,
    state: u32,
) -> glib::gboolean {
    let Some(original) = original() else {
        return 0;
    };
    if original.modern {
        return (original.call)(context, keyval, keycode, state);
    }
    object::g_object_ref(context.cast());
    let connection = gio::g_dbus_proxy_get_connection(context);
    let path = CStr::from_ptr(gio::g_dbus_proxy_get_object_path(context)).to_owned();
    let owner = gio::g_dbus_proxy_get_name_owner(context);
    let sender = if owner.is_null() {
        None
    } else {
        let sender = CStr::from_ptr(owner).to_owned();
        glib::g_free(owner.cast());
        Some(sender)
    };
    let pending = Arc::new(Pending {
        path,
        sender,
        messages: Mutex::new(Some(Vec::new())),
    });
    let filter = gio::g_dbus_connection_add_filter(
        connection,
        Some(collect),
        Arc::into_raw(pending.clone()).cast_mut().cast(),
        Some(release),
    );
    let mut handled = (original.call)(context, keyval, keycode, state);
    // The synchronous reply is a barrier for earlier signals on this connection.
    let messages = pending
        .messages
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
        .unwrap_or_default();
    gio::g_dbus_connection_remove_filter(connection, filter);
    for message in messages {
        let message = message as *mut gio::GDBusMessage;
        let member = gio::g_dbus_message_get_member(message);
        let body = gio::g_dbus_message_get_body(message);
        let mut same = false;
        if matches_string(
            member,
            CStr::from_bytes_with_nul_unchecked(b"ForwardKeyEvent\0"),
        ) && !body.is_null()
            && glib::g_variant_is_of_type(body, b"(uuu)\0".as_ptr().cast()) != 0
        {
            let (mut key, mut code, mut modifiers) = (0u32, 0u32, 0u32);
            glib::g_variant_get(
                body,
                b"(uuu)\0".as_ptr().cast(),
                &mut key,
                &mut code,
                &mut modifiers,
            );
            same = forward_matches(key, code, modifiers, keyval, keycode, state);
        }
        if same {
            handled = 0;
        } else {
            object::g_signal_emit_by_name(
                context.cast(),
                b"g-signal\0".as_ptr().cast(),
                gio::g_dbus_message_get_sender(message),
                member,
                body,
            );
        }
        object::g_object_unref(message.cast());
    }
    object::g_object_unref(context.cast());
    handled
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forwarded_native_key_preserves_synthetic_events() {
        assert!(forward_matches(65, 38, 0, 65, 38, 0));
        assert!(forward_matches(65, 0, 0, 65, 38, 0));
        assert!(!forward_matches(66, 38, 0, 65, 38, 0));
        assert!(!forward_matches(65, 39, 0, 65, 38, 0));
        assert!(!forward_matches(65,38,1,65,38,0));
        assert!(!forward_matches(65,38,1<<30,65,38,0));
        assert!(forward_matches(65,38,(1<<25)|4,65,38,4));
    }
    #[test]
    fn filter_preserves_owned_signal_and_releases_after_delivery() {
        unsafe {
            let path = CString::new("/org/freedesktop/IBus/InputContext_1").unwrap();
            let pending = Arc::new(Pending {
                path: path.clone(),
                sender: None,
                messages: Mutex::new(Some(Vec::new())),
            });
            let message = gio::g_dbus_message_new_signal(
                path.as_ptr(),
                b"org.freedesktop.IBus.InputContext\0".as_ptr().cast(),
                b"CommitText\0".as_ptr().cast(),
            );
            assert!(collect(
                ptr::null_mut(),
                message,
                1,
                Arc::as_ptr(&pending).cast_mut().cast()
            )
            .is_null());
            let owned = pending.messages.lock().unwrap().take().unwrap();
            assert_eq!(owned, vec![message as usize]);
            assert_eq!(
                collect(
                    ptr::null_mut(),
                    message,
                    1,
                    Arc::as_ptr(&pending).cast_mut().cast()
                ),
                message
            );
            object::g_object_unref(message.cast());
        }
    }
}
