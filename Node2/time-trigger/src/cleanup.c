#include "internal.h"

static void cleanup_workloads(struct context *ctx);
static void cleanup_communication(struct context *ctx);
static void cleanup_bpf_trace(void);

void cleanup_context(struct context *ctx)
{
    if (!ctx) return;

    TT_LOG_INFO("Cleaning up resources...");

    cleanup_workloads(ctx);
    cleanup_communication(ctx);
    cleanup_bpf_trace();

    TT_LOG_INFO("Time Trigger shutdown completed.");
}

static void cleanup_workloads(struct context *ctx)
{
    if (!ctx) {
        return;
    }

    struct workload *wl;

    while (!LIST_EMPTY(&ctx->runtime.workloads)) {
        wl = LIST_FIRST(&ctx->runtime.workloads);

        if (!wl) {
            break;  // safety guard
        }

        TT_LOG_INFO("Cleaning up workload: %s", wl->sched_info.workload_id);

        // Clean up tasks in this workload
        struct time_trigger *tt_p;
        while (!LIST_EMPTY(&wl->tt_list)) {
            tt_p = LIST_FIRST(&wl->tt_list);

            if (!tt_p) {
                break;
            }

            // Remove PID from BPF
            bpf_del_pid(tt_p->task.pid);

            // Close pidfd
            if (tt_p->task.pidfd >= 0) {
                close(tt_p->task.pidfd);
            }

            // Delete timer
            timer_delete(tt_p->timer);

            // Remove from list and free memory
            LIST_REMOVE(tt_p, entry);
            TT_FREE(tt_p);
        }

        // Clean up task list from schedule info
        destroy_task_info_list(wl->sched_info.tasks);
        wl->sched_info.tasks = NULL;

        // Clean up hyperperiod timer
        if (wl->hp_manager.hyperperiod_us > 0) {
            timer_delete(wl->hp_manager.hyperperiod_timer);
            log_hyperperiod_statistics(&wl->hp_manager);
        }

        // Remove from workload list and free memory
        LIST_REMOVE(wl, entry);
        TT_FREE(wl);
    }

    ctx->runtime.nr_workloads = 0;
}

static void cleanup_communication(struct context *ctx)
{
    if (ctx->comm.dbus) {
        sd_bus_unref(ctx->comm.dbus);
        ctx->comm.dbus = NULL;
    }

    if (ctx->comm.event) {
        sd_event_unref(ctx->comm.event);
        ctx->comm.event = NULL;
    }
}

static void cleanup_bpf_trace(void)
{
    bpf_off();
}
