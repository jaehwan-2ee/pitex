/* App-local compatibility for pre-1.5.28 IBus GTK modules. The executable
 * exports this public ABI entry point; modern IBus uses its original function.
 * Legacy Hangul forwards unhandled keys while reporting them as consumed.
 * Dispatch commit/preedit signals before returning the original key to GTK.
 * No IM settings or environment variables are changed, and child processes
 * do not inherit the adapter. Only public GObject/GIO/IBus APIs are used. */
#include <gio/gio.h>
#include <dlfcn.h>

typedef struct _IBusInputContext IBusInputContext;
typedef gboolean (*SyncKey)(IBusInputContext *, guint, guint, guint);
typedef void (*AsyncKey)(IBusInputContext *, guint, guint, guint, gint,
                         GCancellable *, GAsyncReadyCallback, gpointer);
typedef gboolean (*FinishKey)(IBusInputContext *, GAsyncResult *, GError **);
static SyncKey original;
static AsyncKey process_async;
static FinishKey finish_async;
static gboolean modern;
static gsize initialized;


typedef struct {
    gboolean done, handled, forwarded;
    guint keyval, keycode, state;
    IBusInputContext *context;
    GArray *handlers;
    gulong own_handler;
} Pending;

static void forward_key(IBusInputContext *context, guint keyval, guint keycode,
                        guint state, Pending *pending) {
    if (keyval == pending->keyval && (keycode == pending->keycode || keycode == 0)) {
        pending->forwarded = TRUE;
        return;
    }
    /* Preserve synthetic keys from other engines using the original handlers. */
    g_signal_handler_block(context, pending->own_handler);
    for (guint i = 0; i < pending->handlers->len; i++)
        g_signal_handler_unblock(context, g_array_index(pending->handlers, gulong, i));
    g_signal_emit_by_name(context, "forward-key-event", keyval, keycode, state);
    for (guint i = 0; i < pending->handlers->len; i++)
        g_signal_handler_block(context, g_array_index(pending->handlers, gulong, i));
    g_signal_handler_unblock(context, pending->own_handler);
}

static void finished(GObject *object, GAsyncResult *result, gpointer data) {
    Pending *pending = data;
    GError *error = NULL;
    pending->handled = finish_async(
        (IBusInputContext *)object, result, &error);
    g_clear_error(&error);
    pending->done = TRUE;
}

gboolean ibus_input_context_process_key_event(IBusInputContext *context,
                                             guint keyval, guint keycode, guint state) {
    if (g_once_init_enter(&initialized)) {
        void *library = dlopen("libibus-1.0.so.5", RTLD_LAZY | RTLD_LOCAL);
        if (library) {
            original = (SyncKey)dlsym(library, "ibus_input_context_process_key_event");
            process_async = (AsyncKey)dlsym(library, "ibus_input_context_process_key_event_async");
            finish_async = (FinishKey)dlsym(library, "ibus_input_context_process_key_event_async_finish");
            modern = dlsym(library, "ibus_input_context_post_process_key_event") != NULL;
        }
        g_once_init_leave(&initialized, 1);
    }
    if (modern && original) return original(context, keyval, keycode, state);
    if (!process_async || !finish_async) return FALSE;
    g_object_ref(context);
    Pending pending = { .keyval = keyval, .keycode = keycode, .state = state, .context = context,
                        .handlers = g_array_new(FALSE, FALSE, sizeof(gulong)) };
    guint signal = g_signal_lookup("forward-key-event", G_OBJECT_TYPE(context));
    gulong handler;
    while ((handler = g_signal_handler_find(context, G_SIGNAL_MATCH_ID | G_SIGNAL_MATCH_UNBLOCKED,
                                             signal, 0, NULL, NULL, NULL))) {
        g_signal_handler_block(context, handler);
        g_array_append_val(pending.handlers, handler);
    }
    pending.own_handler = g_signal_connect(context, "forward-key-event", G_CALLBACK(forward_key), &pending);
    /* Pump IBus replies, but not a second keyboard event from the source
     * currently dispatching this key. Nested key dispatch reverses native
     * insertion order when several keys are already queued. */
    GSource *source = g_main_current_source();
    gboolean can_recurse = source && g_source_get_can_recurse(source);
    if (source) {
        g_source_ref(source);
        g_source_set_can_recurse(source, FALSE);
    }
    process_async(context, keyval, keycode, state, -1, NULL, finished, &pending);
    while (!pending.done) g_main_context_iteration(NULL, TRUE);
    /* GDBus can complete the method before dispatching preceding signals
     * at lower main-loop priority. Keep their handlers active through both. */
    while (g_main_context_pending(NULL)) g_main_context_iteration(NULL, FALSE);
    if (source) {
        g_source_set_can_recurse(source, can_recurse);
        g_source_unref(source);
    }
    g_signal_handler_disconnect(context, pending.own_handler);
    for (guint i = 0; i < pending.handlers->len; i++)
        if (g_signal_handler_is_connected(context, g_array_index(pending.handlers, gulong, i)))
            g_signal_handler_unblock(context, g_array_index(pending.handlers, gulong, i));
    g_array_unref(pending.handlers);
    g_object_unref(context);
    return pending.handled && !pending.forwarded;
}
