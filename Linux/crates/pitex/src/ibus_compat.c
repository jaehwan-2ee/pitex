/* App-local compatibility for pre-1.5.28 IBus GTK4 modules. Export the
 * public synchronous entry point; modern IBus keeps its original behavior.
 * Legacy engines send commit/preedit/forward signals before the key reply,
 * but GTK sees them too late. Collect those messages during the synchronous
 * call and deliver them on the UI thread before returning the native key.
 * No nested GTK loop, private IBus layouts, or system IM setting changes. */
#include <gio/gio.h>
#include <dlfcn.h>

typedef struct _IBusInputContext IBusInputContext;
typedef gboolean (*SyncKey)(IBusInputContext *, guint, guint, guint);
static SyncKey original;
static gboolean modern;
static gsize initialized;

void pitex_ibus_initialize(void) {
    /* GTK4 cannot reinject arbitrary unhandled keys in async mode. Undo an
     * old IBUS_ENABLE_SYNC_MODE=0 workaround for this process only. */
    g_setenv("IBUS_ENABLE_SYNC_MODE", "1", TRUE);
}

typedef struct {
    gatomicrefcount refs;
    GMutex mutex;
    gchar *path, *sender;
    GPtrArray *messages;
} Pending;

static void pending_unref(gpointer data) {
    Pending *pending = data;
    if (!g_atomic_ref_count_dec(&pending->refs)) return;
    g_clear_pointer(&pending->messages, g_ptr_array_unref);
    g_free(pending->path);
    g_free(pending->sender);
    g_mutex_clear(&pending->mutex);
    g_free(pending);
}

/* GDBus runs filters on its worker thread, even during a synchronous call.
 * Taking ownership here prevents the usual deferred UI-thread delivery. */
static GDBusMessage *collect_signal(GDBusConnection *connection,
                                    GDBusMessage *message, gboolean incoming,
                                    gpointer data) {
    (void)connection;
    Pending *pending = data;
    if (!incoming || g_dbus_message_get_message_type(message) != G_DBUS_MESSAGE_TYPE_SIGNAL ||
        g_strcmp0(g_dbus_message_get_path(message), pending->path) ||
        g_strcmp0(g_dbus_message_get_interface(message), "org.freedesktop.IBus.InputContext") ||
        (pending->sender && g_strcmp0(g_dbus_message_get_sender(message), pending->sender)))
        return message;
    g_mutex_lock(&pending->mutex);
    if (pending->messages) {
        g_ptr_array_add(pending->messages, message);
        message = NULL;
    }
    g_mutex_unlock(&pending->mutex);
    return message;
}

gboolean ibus_input_context_process_key_event(IBusInputContext *context,
                                             guint keyval, guint keycode, guint state) {
    if (g_once_init_enter(&initialized)) {
        void *library = dlopen("libibus-1.0.so.5", RTLD_LAZY | RTLD_LOCAL);
        if (library) {
            original = (SyncKey)dlsym(library, "ibus_input_context_process_key_event");
            modern = dlsym(library, "ibus_input_context_post_process_key_event") != NULL;
        }
        g_once_init_leave(&initialized, 1);
    }
    if (!original) return FALSE;
    if (modern) return original(context, keyval, keycode, state);
    GDBusProxy *proxy = G_DBUS_PROXY(g_object_ref(context));
    GDBusConnection *connection = g_dbus_proxy_get_connection(proxy);
    Pending *pending = g_new0(Pending, 1);
    g_atomic_ref_count_init(&pending->refs);
    g_mutex_init(&pending->mutex);
    pending->path = g_strdup(g_dbus_proxy_get_object_path(proxy));
    pending->sender = g_dbus_proxy_get_name_owner(proxy);
    pending->messages = g_ptr_array_new_with_free_func(g_object_unref);
    g_atomic_ref_count_inc(&pending->refs);
    guint filter = g_dbus_connection_add_filter(connection, collect_signal, pending, pending_unref);
    gboolean handled = original(context, keyval, keycode, state);
    /* The reply is a barrier for preceding signals on this connection. */
    g_mutex_lock(&pending->mutex);
    GPtrArray *messages = pending->messages;
    pending->messages = NULL;
    g_mutex_unlock(&pending->mutex);
    g_dbus_connection_remove_filter(connection, filter);
    for (guint i = 0; i < messages->len; i++) {
        GDBusMessage *message = g_ptr_array_index(messages, i);
        const gchar *member = g_dbus_message_get_member(message);
        GVariant *body = g_dbus_message_get_body(message);
        if (g_strcmp0(member, "ForwardKeyEvent") == 0 &&
            body && g_variant_is_of_type(body, G_VARIANT_TYPE("(uuu)"))) {
            guint forwarded_key, forwarded_code, forwarded_state;
            g_variant_get(body, "(uuu)", &forwarded_key, &forwarded_code, &forwarded_state);
            if (forwarded_key == keyval && (forwarded_code == keycode || forwarded_code == 0)) {
                handled = FALSE;
                continue;
            }
        }
        /* Reuse IBus's public GDBusProxy decoder for all other signals,
         * including synthetic keys and surrounding-text edits. */
        g_signal_emit_by_name(proxy, "g-signal", g_dbus_message_get_sender(message), member, body);
    }
    g_ptr_array_unref(messages);
    pending_unref(pending);
    g_object_unref(proxy);
    return handled;
}
