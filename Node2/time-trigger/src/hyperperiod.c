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
    hp_mgr->ctx = ctx;  // Set context pointer

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

    // Quick NULL check
    if (unlikely(!hp_mgr || hp_mgr->hyperperiod_start_time_us == 0)) {
        return 0;
    }

    clock_gettime(hp_mgr->ctx->config.clockid, &now);
    uint64_t current_time_us = tt_timespec_to_us(&now);

    uint64_t elapsed_us = current_time_us - hp_mgr->hyperperiod_start_time_us;

    // Optimize modulo with bitwise operation (when power of 2)
    if (hp_mgr->hyperperiod_us & (hp_mgr->hyperperiod_us - 1)) {
        // Regular modulo operation
        return elapsed_us % hp_mgr->hyperperiod_us;
    } else {
        // Use bitmask for power of 2
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

tt_error_t start_hyperperiod_timer(struct workload *wl)
{
    struct itimerspec its;
    struct sigevent sev;
    struct context *ctx = wl->ctx;
    struct hyperperiod_manager *hp_mgr = &wl->hp_manager;

    if (hp_mgr->hyperperiod_us == 0) {
        TT_LOG_WARNING("Hyperperiod not set for workload %s, skipping hyperperiod timer",
            hp_mgr->workload_id);
        return TT_SUCCESS;
    }

    // Set hyperperiod start time to match with task timers
    hp_mgr->hyperperiod_start_ts = ctx->runtime.starttimer_ts;
    hp_mgr->hyperperiod_start_time_us = ts_us(hp_mgr->hyperperiod_start_ts);

    TT_LOG_INFO("Workload %s: Hyperperiod start time set: %lu us",
        hp_mgr->workload_id, hp_mgr->hyperperiod_start_time_us);

    memset(&sev, 0, sizeof(sev));
    memset(&its, 0, sizeof(its));

    sev.sigev_notify = SIGEV_THREAD;
    sev.sigev_notify_function = hyperperiod_cycle_handler;
    sev.sigev_value.sival_ptr = hp_mgr;

    // Set hyperperiod cycle interval
    its.it_value.tv_sec = ctx->runtime.starttimer_ts.tv_sec + (hp_mgr->hyperperiod_us / USEC_PER_SEC);
    its.it_value.tv_nsec = ctx->runtime.starttimer_ts.tv_nsec + (hp_mgr->hyperperiod_us % USEC_PER_SEC) * NSEC_PER_USEC;
    if (its.it_value.tv_nsec >= NSEC_PER_SEC) {
        its.it_value.tv_sec++;
        its.it_value.tv_nsec -= NSEC_PER_SEC;
    }

    its.it_interval.tv_sec = hp_mgr->hyperperiod_us / USEC_PER_SEC;
    its.it_interval.tv_nsec = (hp_mgr->hyperperiod_us % USEC_PER_SEC) * NSEC_PER_USEC;

    TT_LOG_INFO("Workload %s: Starting hyperperiod timer: %lu us interval (%lds %ldns)",
        hp_mgr->workload_id, hp_mgr->hyperperiod_us, its.it_interval.tv_sec, its.it_interval.tv_nsec);

    if (timer_create(ctx->config.clockid, &sev, &hp_mgr->hyperperiod_timer)) {
        perror("Failed to create hyperperiod timer");
        return TT_ERROR_TIMER;
    }

    if (timer_settime(hp_mgr->hyperperiod_timer, TIMER_ABSTIME, &its, NULL)) {
        perror("Failed to start hyperperiod timer");
        return TT_ERROR_TIMER;
    }

    return TT_SUCCESS;
}
