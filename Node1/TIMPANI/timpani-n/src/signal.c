#include "internal.h"

static struct context *g_ctx = NULL;

static void signal_handler(int signo)
{
    if (g_ctx) {
        g_ctx->runtime.shutdown_requested = 1;
        TT_LOG_INFO("Shutdown signal received: %d", signo);
    }
}

tt_error_t setup_signal_handlers(struct context *ctx)
{
    g_ctx = ctx;

    struct sigaction sa;
    sa.sa_handler = signal_handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;

    if (sigaction(SIGINT, &sa, NULL) < 0) {
        perror("Failed to set SIGINT handler");
        return TT_ERROR_SIGNAL;
    }

    if (sigaction(SIGTERM, &sa, NULL) < 0) {
        perror("Failed to set SIGTERM handler");
        return TT_ERROR_SIGNAL;
    }

    return TT_SUCCESS;
}
