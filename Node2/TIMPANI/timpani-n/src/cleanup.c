/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include "internal.h"

static void cleanup_tasks(struct context *ctx);
static void cleanup_communication(struct context *ctx);
static void cleanup_hyperperiod(struct context *ctx);
static void cleanup_bpf_trace(void);

void cleanup_context(struct context *ctx)
{
    if (!ctx) return;

    TT_LOG_INFO("Cleaning up resources...");

    cleanup_tasks(ctx);
    cleanup_communication(ctx);
    cleanup_hyperperiod(ctx);
    cleanup_bpf_trace();

    TT_LOG_INFO("Time Trigger shutdown completed.");
}

static void cleanup_tasks(struct context *ctx)
{
    if (!ctx) {
        return;
    }

    struct time_trigger *tt_p;

    while (!LIST_EMPTY(&ctx->runtime.tt_list)) {
        tt_p = LIST_FIRST(&ctx->runtime.tt_list);

        if (!tt_p) {
            break;  // 안전장치
        }

        // BPF에서 PID 제거
        bpf_del_pid(tt_p->task.pid);

        // pidfd 닫기
        if (tt_p->task.pidfd >= 0) {
            close(tt_p->task.pidfd);
        }

        // 타이머 삭제
        timer_delete(tt_p->timer);

        // 리스트에서 제거 및 메모리 해제
        LIST_REMOVE(tt_p, entry);
        TT_FREE(tt_p);
    }

    // 스케줄 정보의 태스크 리스트 정리
    destroy_task_info_list(ctx->runtime.sched_info.tasks);
    ctx->runtime.sched_info.tasks = NULL;
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

    pthread_mutex_destroy(&ctx->comm.dbus_mutex);
}

static void cleanup_hyperperiod(struct context *ctx)
{
    if (ctx->hp_manager.hyperperiod_us > 0) {
        timer_delete(ctx->hp_manager.hyperperiod_timer);
        log_hyperperiod_statistics(&ctx->hp_manager);
    }
}

static void cleanup_bpf_trace(void)
{
    bpf_off();
}
