/* Experimental app-local adapter for the pre-1.5.28 IBus synchronous API.
 * Preserve commit/preedit ordering and return unhandled original keys to GTK.
 * No user settings are changed. */
#include <ibus.h>

typedef struct {
    gboolean done, handled, forwarded;
    guint keyval, keycode, state;
    IBusInputContext *context;
    GArray *handlers;
    gulong own_handler;
} Pending;

static void forward_key(IBusInputContext *context, guint keyval, guint keycode,
                        guint state, Pending *pending) {
    if (keyval == pending->keyval && keycode == pending->keycode) {
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
    pending->handled = ibus_input_context_process_key_event_async_finish(
        IBUS_INPUT_CONTEXT(object), result, &error);
    g_clear_error(&error);
    pending->done = TRUE;
}

gboolean ibus_input_context_process_key_event(IBusInputContext *context,
                                             guint keyval, guint keycode, guint state) {
    Pending pending = { .keyval = keyval, .keycode = keycode, .state = state, .context = context,
                        .handlers = g_array_new(FALSE, FALSE, sizeof(gulong)) };
    guint signal = g_signal_lookup("forward-key-event", IBUS_TYPE_INPUT_CONTEXT);
    gulong handler;
    while ((handler = g_signal_handler_find(context, G_SIGNAL_MATCH_ID | G_SIGNAL_MATCH_UNBLOCKED,
                                             signal, 0, NULL, NULL, NULL))) {
        g_signal_handler_block(context, handler);
        g_array_append_val(pending.handlers, handler);
    }
    pending.own_handler = g_signal_connect(context, "forward-key-event", G_CALLBACK(forward_key), &pending);
    ibus_input_context_process_key_event_async(context, keyval, keycode, state, -1, NULL, finished, &pending);
    while (!pending.done) g_main_context_iteration(NULL, TRUE);
    g_signal_handler_disconnect(context, pending.own_handler);
    for (guint i = 0; i < pending.handlers->len; i++)
        g_signal_handler_unblock(context, g_array_index(pending.handlers, gulong, i));
    g_array_unref(pending.handlers);
    return pending.handled && !pending.forwarded;
}
