/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include "internal.h"

void destroy_task_info_list(struct task_info *tasks)
{
    while (tasks) {
        struct task_info *current = tasks;
        tasks = tasks->next;
        TT_FREE(current);
    }
}

static struct time_trigger *task_create_node(struct task_info *ti, struct context *ctx)
{
    struct time_trigger *tt_node = calloc(1, sizeof(struct time_trigger));
    if (!tt_node) {
        TT_LOG_ERROR("Failed to allocate memory for time_trigger");
        return NULL;
    }

    memcpy(&tt_node->task, ti, sizeof(tt_node->task));
    tt_node->ctx = ctx;  // context 포인터 설정
    return tt_node;
}

static tt_error_t task_setup_process(struct time_trigger *tt_node)
{
    int pid;
    ttsched_error_t pid_result = get_pid_by_name(tt_node->task.name, &pid);
    if (pid_result != TTSCHED_SUCCESS) {
        TT_LOG_INFO("%s is not running! (%s)", tt_node->task.name, ttsched_error_string(pid_result));
        return TT_ERROR_CONFIG;
    }

    // Apply affinity to ALL threads so DDS/RTPS worker threads also move CPUs.
    // set_affinity_cpumask only sets the main thread; DDS executor threads
    // (Dust DDS Execut, RTPS user defin, etc.) would stay on the old CPU
    // and cause DDS take() latency to remain high even after the affinity change.
    ttsched_error_t affinity_result = set_affinity_cpumask_all_threads(pid, tt_node->task.cpu_affinity);
    if (affinity_result != TTSCHED_SUCCESS) {
        TT_LOG_WARNING("Failed to set CPU affinity for task %s (PID %d): %s",
            tt_node->task.name, pid, ttsched_error_string(affinity_result));
        // Continue anyway, affinity is not critical for basic operation
    }

    ttsched_error_t sched_result = set_schedattr(pid, tt_node->task.sched_priority, tt_node->task.sched_policy);
    if (sched_result != TTSCHED_SUCCESS) {
        TT_LOG_WARNING("Failed to set scheduling attributes for task %s (PID %d): %s",
            tt_node->task.name, pid, ttsched_error_string(sched_result));
        // Continue anyway, scheduling priority is not critical for basic operation
    }

    tt_node->task.pid = pid;

    // Create pidfd for the task
    ttsched_error_t pidfd_result = create_pidfd(pid, &tt_node->task.pidfd);
    if (pidfd_result != TTSCHED_SUCCESS) {
        TT_LOG_ERROR("Failed to create pidfd for task %s (PID %d): %s",
            tt_node->task.name, pid, ttsched_error_string(pidfd_result));
        return TT_ERROR_CONFIG;
    }

    if (bpf_add_pid(pid) < 0) {
        TT_LOG_WARNING("Failed to add PID %d to BPF monitoring", pid);
        // Continue anyway, monitoring is not critical for basic operation
    }

    return TT_SUCCESS;
}

/*
 * Start a POSIX interval timer for a single time_trigger node that was added
 * dynamically (after start_timers() has already fired for the initial list).
 *
 * Unlike the initial start_timers() path, we use CLOCK_REALTIME "now" as the
 * first-fire time so the task begins receiving SIGNO_TT immediately rather
 * than waiting for the original (now past) starttimer_ts.
 *
 * The workload_id is recorded in the task name field for log correlation only;
 * the task runs its own independent periodic timer at tt_node->task.period and
 * is intentionally NOT counted in the existing hp_manager — it belongs to a
 * separate workload/hyperperiod.
 */
static tt_error_t start_task_timer(struct time_trigger *tt_node)
{
    struct itimerspec its;
    struct sigevent   sev;
    struct timespec   now;

    memset(&sev, 0, sizeof(sev));
    memset(&its, 0, sizeof(its));

    /* Fire first time slightly in the future from now, then repeat at period */
    clock_gettime(tt_node->ctx->config.clockid, &now);
    now.tv_nsec += TT_TIMER_INCREMENT_NS;
    if (now.tv_nsec >= (long)NSEC_PER_SEC) {
        now.tv_sec++;
        now.tv_nsec -= NSEC_PER_SEC;
    }

    sev.sigev_notify          = SIGEV_THREAD;
    sev.sigev_notify_function = timer_expired_handler;
    sev.sigev_value.sival_ptr = tt_node;

    its.it_value    = now;
    its.it_interval.tv_sec  = tt_node->task.period / USEC_PER_SEC;
    its.it_interval.tv_nsec = tt_node->task.period % USEC_PER_SEC * NSEC_PER_USEC;

    TT_LOG_INFO("[start_task_timer] creating timer for '%s' (PID %d): "
                "period=%d us (%lds %ldns)",
                tt_node->task.name, tt_node->task.pid, tt_node->task.period,
                its.it_interval.tv_sec, its.it_interval.tv_nsec);

    if (timer_create(tt_node->ctx->config.clockid, &sev, &tt_node->timer) != 0) {
        TT_LOG_ERROR("[start_task_timer] timer_create failed for '%s': %s",
                     tt_node->task.name, strerror(errno));
        return TT_ERROR_TIMER;
    }

    if (timer_settime(tt_node->timer, TIMER_ABSTIME, &its, NULL) != 0) {
        TT_LOG_ERROR("[start_task_timer] timer_settime failed for '%s': %s",
                     tt_node->task.name, strerror(errno));
        timer_delete(tt_node->timer);
        return TT_ERROR_TIMER;
    }

    TT_LOG_INFO("[start_task_timer] timer armed for '%s' — first fire in ~%ldms",
                tt_node->task.name, TT_TIMER_INCREMENT_NS / 1000000);
    return TT_SUCCESS;
}

/*
 * Scan new_sinfo for tasks that belong to this node but are not yet tracked
 * in tt_list (i.e. tasks from a second workload submitted after start-up).
 *
 * For each such task:
 *   - resolve the process, set its CPU affinity and scheduling attributes
 *   - create a time_trigger entry and insert it into tt_list
 *   - register its pidfd with the supplied epoll fd so that process
 *     termination events are caught by epoll_loop()
 *
 * Individual task failures are non-fatal and logged; the loop continues with
 * remaining tasks.  task_setup_process() will return TT_ERROR_CONFIG when the
 * process is not yet running, so unresolved tasks will be retried automatically
 * on the next SCHED_POLL_INTERVAL_SEC tick.
 *
 * Returns the number of newly registered tasks (>= 0).
 */
int register_new_tasks(struct context *ctx, struct sched_info *new_sinfo, int efd)
{
    int added = 0;
    int total_in_sinfo = 0;
    int skipped_node_mismatch = 0;
    int skipped_already_tracked = 0;

    /* Count total tasks received for diagnostics */
    for (struct task_info *t = new_sinfo->tasks; t; t = t->next)
        total_in_sinfo++;

    TT_LOG_INFO("[register_new_tasks] poll returned %d task(s) from workload='%s', "
                "this node_id='%s'",
                total_in_sinfo, new_sinfo->workload_id, ctx->config.node_id);

    if (total_in_sinfo == 0) {
        /* timpani-o returned an empty task list – nothing to do */
        TT_LOG_WARNING("[register_new_tasks] new_sinfo has 0 tasks – "
                       "check if timpani-o has the workload stored");
        return 0;
    }

    for (struct task_info *ti = new_sinfo->tasks; ti; ti = ti->next) {
        TT_LOG_DEBUG("[register_new_tasks] inspecting task '%s' node_id='%s' "
                     "affinity=0x%lx policy=%d prio=%d period=%d",
                     ti->name, ti->node_id, ti->cpu_affinity,
                     ti->sched_policy, ti->sched_priority, ti->period);

        /* Only handle tasks that target this node */
        if (strcmp(ctx->config.node_id, ti->node_id) != 0) {
            TT_LOG_DEBUG("[register_new_tasks] skipping '%s': node_id mismatch "
                         "(task='%s' vs this='%s')",
                         ti->name, ti->node_id, ctx->config.node_id);
            skipped_node_mismatch++;
            continue;
        }

        /* Check if this task is already tracked in tt_list */
        bool already_tracked = false;
        struct time_trigger *tt_p;
        LIST_FOREACH(tt_p, &ctx->runtime.tt_list, entry) {
            if (strcmp(tt_p->task.name, ti->name) == 0) {
                already_tracked = true;
                break;
            }
        }

        if (already_tracked) {
            TT_LOG_DEBUG("[register_new_tasks] skipping '%s': already in tt_list "
                         "(PID %d, affinity=0x%lx)",
                         ti->name, tt_p->task.pid, tt_p->task.cpu_affinity);
            skipped_already_tracked++;
            continue;
        }

        TT_LOG_INFO("[register_new_tasks] NEW task detected: '%s' (workload='%s', "
                    "node_id='%s') – attempting to register as independent hyperperiod",
                    ti->name, new_sinfo->workload_id, ti->node_id);

        /* Allocate and initialise a new time_trigger node using the same
         * helper used during start-up in init_task_list(). */
        struct time_trigger *tt_node = task_create_node(ti, ctx);
        if (!tt_node) {
            TT_LOG_ERROR("[register_new_tasks] calloc failed for task '%s'", ti->name);
            /* Non-fatal: try the remaining tasks */
            continue;
        }

        /* Resolve PID, set affinity / sched attrs, create pidfd.
         * If the process is not yet running, task_setup_process returns
         * TT_ERROR_CONFIG and we discard the node; it will be retried on the
         * next poll tick once the process has started. */
        TT_LOG_INFO("[register_new_tasks] calling task_setup_process for '%s'", ti->name);
        if (task_setup_process(tt_node) != TT_SUCCESS) {
            TT_LOG_WARNING("[register_new_tasks] task_setup_process FAILED for '%s' – "
                           "process may not be running yet, will retry next poll cycle",
                           ti->name);
            TT_FREE(tt_node);
            continue;
        }

        TT_LOG_INFO("[register_new_tasks] task '%s' resolved to PID %d, "
                    "affinity=0x%lx applied",
                    tt_node->task.name, tt_node->task.pid, tt_node->task.cpu_affinity);

        /* Start an independent POSIX interval timer for this task.
         * This is the critical step that ensures the task actually receives
         * SIGNO_TT signals at its own period — completely separate from the
         * existing workload's hyperperiod.  start_timers() was already called
         * at start-up and only covered the initial tt_list, so we must arm
         * a fresh timer here. */
        if (start_task_timer(tt_node) != TT_SUCCESS) {
            TT_LOG_ERROR("[register_new_tasks] failed to start timer for '%s' – "
                         "task will NOT receive TT signals; skipping",
                         ti->name);
            TT_FREE(tt_node);
            continue;
        }

        /* Register the process's pidfd with epoll so that its termination is
         * detected the same way as tasks registered during start-up. */
        struct epoll_event ev;
        ev.events  = EPOLLIN;
        ev.data.fd = tt_node->task.pidfd;
        if (epoll_ctl(efd, EPOLL_CTL_ADD, tt_node->task.pidfd, &ev) < 0) {
            TT_LOG_ERROR("[register_new_tasks] epoll_ctl(ADD) failed for '%s' "
                         "(pidfd=%d): %s",
                         tt_node->task.name, tt_node->task.pidfd, strerror(errno));
            /* Non-fatal: the task still receives TT signals; we just won't
             * detect its termination via epoll.  Insert it anyway. */
        } else {
            TT_LOG_INFO("[register_new_tasks] pidfd %d for '%s' added to epoll",
                        tt_node->task.pidfd, tt_node->task.name);
        }

        /* Insert at the head of the live task list.
         * NOTE: we do NOT increment hp_manager.tasks_in_hyperperiod here because
         * this task belongs to a different workload ('new_sinfo->workload_id') with
         * its own independent hyperperiod — incrementing the existing hp_manager
         * counter would corrupt its statistics and cycle tracking. */
        LIST_INSERT_HEAD(&ctx->runtime.tt_list, tt_node, entry);

        TT_LOG_INFO("[register_new_tasks] SUCCESS: task '%s' (PID %d) from workload "
                    "'%s' inserted into tt_list with its own timer (period=%d us). "
                    "Existing hyperperiod '%s' is unaffected.",
                    tt_node->task.name, tt_node->task.pid,
                    new_sinfo->workload_id, tt_node->task.period,
                    ctx->hp_manager.workload_id);
        added++;
    }

    TT_LOG_INFO("[register_new_tasks] summary: total=%d node_mismatch=%d "
                "already_tracked=%d newly_added=%d",
                total_in_sinfo, skipped_node_mismatch,
                skipped_already_tracked, added);

    return added;
}

/*
 * For each running task in tt_list, check if its cpu_affinity has changed
 * in new_sinfo.  If so, reapply the affinity using the full bitmask.
 */
void reapply_affinities(struct context *ctx, struct sched_info *new_sinfo)
{
    struct time_trigger *tt_p;

    LIST_FOREACH(tt_p, &ctx->runtime.tt_list, entry) {
        struct task_info *ti;
        for (ti = new_sinfo->tasks; ti; ti = ti->next) {
            if (strcmp(tt_p->task.name, ti->name) != 0)
                continue;
            if (tt_p->task.cpu_affinity == ti->cpu_affinity)
                break;
            TT_LOG_INFO("Updating affinity for task %s (PID %d): "
                        "0x%lx -> 0x%lx",
                        tt_p->task.name, tt_p->task.pid,
                        tt_p->task.cpu_affinity, ti->cpu_affinity);
            // Apply to ALL threads so DDS/RTPS worker threads also migrate.
            set_affinity_cpumask_all_threads(tt_p->task.pid, ti->cpu_affinity);
            tt_p->task.cpu_affinity = ti->cpu_affinity;
            break;
        }
    }
}

tt_error_t init_task_list(struct context *ctx)
{
    int success_count = 0;

    // LIST_INIT는 config_set_defaults에서 이미 호출됨

    for (struct task_info *ti = ctx->runtime.sched_info.tasks; ti; ti = ti->next) {
        if (strcmp(ctx->config.node_id, ti->node_id) != 0) {
            /* The task does not belong to this node. */
            continue;
        }

        struct time_trigger *tt_node = task_create_node(ti, ctx);
        if (!tt_node) {
            continue;
        }

        if (task_setup_process(tt_node) != TT_SUCCESS) {
            TT_FREE(tt_node);
            continue;
        }

        LIST_INSERT_HEAD(&ctx->runtime.tt_list, tt_node, entry);

        // Count tasks for hyperperiod management
        ctx->hp_manager.tasks_in_hyperperiod++;

        success_count++;
    }

    if (success_count == 0) {
        TT_LOG_ERROR("No tasks were successfully initialized");
        return TT_ERROR_CONFIG;
    }

    TT_LOG_INFO("Successfully initialized %d tasks", success_count);
    return TT_SUCCESS;
}
