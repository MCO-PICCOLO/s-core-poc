/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include "internal.h"

tt_error_t init_hyperperiod(struct context *ctx, const char *workload_id, uint64_t hyperperiod_us, struct hyperperiod_manager *hp_mgr)
{
    if (!ctx || !workload_id || !hp_mgr) {
        return TT_ERROR_CONFIG;
    }

    strncpy(hp_mgr->workload_id, workload_id, sizeof(hp_mgr->workload_id) - 1);
    hp_mgr->hyperperiod_us = hyperperiod_us;
    hp_mgr->current_cycle = 0;
    hp_mgr->completed_cycles = 0;
    hp_mgr->total_deadline_misses = 0;
    hp_mgr->cycle_deadline_misses = 0;
    hp_mgr->tasks_in_hyperperiod = 0;
    hp_mgr->ctx = ctx;  // Context 포인터 설정

    // Hyperperiod start time will be set when timers actually start
    hp_mgr->hyperperiod_start_time_us = 0;

    TT_LOG_INFO("Hyperperiod Manager initialized:");
    TT_LOG_INFO("  Workload ID: %s", hp_mgr->workload_id);
    TT_LOG_INFO("  Hyperperiod: %lu us (%.3f ms)",
           hp_mgr->hyperperiod_us, hp_mgr->hyperperiod_us / 1000.0);
    TT_LOG_INFO("  Start time will be set when timers start");    return TT_SUCCESS;
}

void hyperperiod_cycle_handler(union sigval value)
{
    struct hyperperiod_manager *hp_mgr = (struct hyperperiod_manager *)value.sival_ptr;
    struct timespec now;
    uint64_t cycle_time_us;

    clock_gettime(hp_mgr->ctx->config.clockid, &now);
    cycle_time_us = ts_us(now);

    // Update cycle information
    hp_mgr->completed_cycles++;
    hp_mgr->current_cycle = (hp_mgr->current_cycle + 1) %
        ((hp_mgr->hyperperiod_us > 0) ? 1 : 1); // Will be used for multi-cycle tracking

    TT_LOG_DEBUG("Hyperperiod cycle %lu completed at %lu us, deadline misses in this cycle: %u",
        hp_mgr->completed_cycles, cycle_time_us, hp_mgr->cycle_deadline_misses);

#ifdef HP_DEBUG
    TT_LOG_INFO("Hyperperiod cycle %lu completed (total misses: %u, cycle misses: %u)",
        hp_mgr->completed_cycles, hp_mgr->total_deadline_misses, hp_mgr->cycle_deadline_misses);
#endif

    // Reset cycle-specific counters
    hp_mgr->cycle_deadline_misses = 0;

    // Log statistics every interval
    if (hp_mgr->completed_cycles % TT_STATISTICS_LOG_INTERVAL == 0) {
        log_hyperperiod_statistics(hp_mgr);
    }
}

uint64_t get_hyperperiod_relative_time(const struct hyperperiod_manager *hp_mgr)
{
    struct timespec now;

    // 빠른 NULL 검사
    if (unlikely(!hp_mgr || hp_mgr->hyperperiod_start_time_us == 0)) {
        return 0;
    }

    clock_gettime(hp_mgr->ctx->config.clockid, &now);
    uint64_t current_time_us = tt_timespec_to_us(&now);

    uint64_t elapsed_us = current_time_us - hp_mgr->hyperperiod_start_time_us;

    // 비트 연산으로 모듈로 연산 최적화 (2의 거듭제곱일 때)
    if (hp_mgr->hyperperiod_us & (hp_mgr->hyperperiod_us - 1)) {
        // 일반적인 모듈로 연산
        return elapsed_us % hp_mgr->hyperperiod_us;
    } else {
        // 2의 거듭제곱인 경우 비트 마스크 사용
        return elapsed_us & (hp_mgr->hyperperiod_us - 1);
    }
}

void log_hyperperiod_statistics(const struct hyperperiod_manager *hp_mgr)
{
    double miss_rate = hp_mgr->completed_cycles > 0 ?
        (double)hp_mgr->total_deadline_misses / hp_mgr->completed_cycles : 0.0;

    TT_LOG_INFO("=== Hyperperiod Statistics ===");
    TT_LOG_INFO("Workload: %s", hp_mgr->workload_id);
    TT_LOG_INFO("Completed cycles: %lu", hp_mgr->completed_cycles);
    TT_LOG_INFO("Hyperperiod length: %lu us", hp_mgr->hyperperiod_us);
    TT_LOG_INFO("Total deadline misses: %u", hp_mgr->total_deadline_misses);
    TT_LOG_INFO("Miss rate per cycle: %.4f", miss_rate);
    TT_LOG_INFO("Tasks in hyperperiod: %u", hp_mgr->tasks_in_hyperperiod);
    TT_LOG_INFO("==============================");
}

tt_error_t start_hyperperiod_timer(struct context *ctx)
{
    struct itimerspec its;
    struct sigevent sev;

    if (ctx->hp_manager.hyperperiod_us == 0) {
        TT_LOG_WARNING("Hyperperiod not set, skipping hyperperiod timer");
        return TT_SUCCESS;
    }

    // Set hyperperiod start time to match with task timers
    ctx->hp_manager.hyperperiod_start_ts = ctx->runtime.starttimer_ts;
    ctx->hp_manager.hyperperiod_start_time_us = ts_us(ctx->hp_manager.hyperperiod_start_ts);

    TT_LOG_INFO("Hyperperiod start time set: %lu us", ctx->hp_manager.hyperperiod_start_time_us);

    memset(&sev, 0, sizeof(sev));
    memset(&its, 0, sizeof(its));

    sev.sigev_notify = SIGEV_THREAD;
    sev.sigev_notify_function = hyperperiod_cycle_handler;
    sev.sigev_value.sival_ptr = &ctx->hp_manager;

    // Set hyperperiod cycle interval
    its.it_value.tv_sec = ctx->runtime.starttimer_ts.tv_sec + (ctx->hp_manager.hyperperiod_us / USEC_PER_SEC);
    its.it_value.tv_nsec = ctx->runtime.starttimer_ts.tv_nsec + (ctx->hp_manager.hyperperiod_us % USEC_PER_SEC) * NSEC_PER_USEC;
    if (its.it_value.tv_nsec >= NSEC_PER_SEC) {
        its.it_value.tv_sec++;
        its.it_value.tv_nsec -= NSEC_PER_SEC;
    }

    its.it_interval.tv_sec = ctx->hp_manager.hyperperiod_us / USEC_PER_SEC;
    its.it_interval.tv_nsec = (ctx->hp_manager.hyperperiod_us % USEC_PER_SEC) * NSEC_PER_USEC;

    TT_LOG_INFO("Starting hyperperiod timer: %lu us interval (%lds %ldns)",
        ctx->hp_manager.hyperperiod_us, its.it_interval.tv_sec, its.it_interval.tv_nsec);

    if (timer_create(ctx->config.clockid, &sev, &ctx->hp_manager.hyperperiod_timer)) {
        perror("Failed to create hyperperiod timer");
        return TT_ERROR_TIMER;
    }

    if (timer_settime(ctx->hp_manager.hyperperiod_timer, TIMER_ABSTIME, &its, NULL)) {
        perror("Failed to start hyperperiod timer");
        return TT_ERROR_TIMER;
    }

    return TT_SUCCESS;
}
