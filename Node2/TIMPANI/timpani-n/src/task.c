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
