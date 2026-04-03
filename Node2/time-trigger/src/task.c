#include "internal.h"

void destroy_task_info_list(struct task_info *tasks)
{
    while (tasks) {
        struct task_info *current = tasks;
        tasks = tasks->next;
        TT_FREE(current);
    }
}

static struct time_trigger *task_create_node(struct task_info *ti, struct workload *wl)
{
    struct time_trigger *tt_node = calloc(1, sizeof(struct time_trigger));
    if (!tt_node) {
        TT_LOG_ERROR("Failed to allocate memory for time_trigger");
        return NULL;
    }

    memcpy(&tt_node->task, ti, sizeof(tt_node->task));
    tt_node->ctx = wl->ctx;       // set context pointer
    tt_node->workload = wl;       // set owning workload pointer
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

    ttsched_error_t affinity_result = set_affinity(pid, (int)tt_node->task.cpu_affinity);
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

tt_error_t init_task_list(struct workload *wl)
{
    int success_count = 0;
    struct context *ctx = wl->ctx;

    TT_LOG_INFO("Initializing tasks for workload: %s", wl->sched_info.workload_id);

    for (struct task_info *ti = wl->sched_info.tasks; ti; ti = ti->next) {
        if (strcmp(ctx->config.node_id, ti->node_id) != 0) {
            /* The task does not belong to this node. */
            continue;
        }

        struct time_trigger *tt_node = task_create_node(ti, wl);
        if (!tt_node) {
            continue;
        }

        if (task_setup_process(tt_node) != TT_SUCCESS) {
            TT_FREE(tt_node);
            continue;
        }

        LIST_INSERT_HEAD(&wl->tt_list, tt_node, entry);

        // Count tasks for hyperperiod management
        wl->hp_manager.tasks_in_hyperperiod++;

        success_count++;
    }

    wl->nr_active_tasks = success_count;

    TT_LOG_INFO("Successfully initialized %d tasks for workload %s",
        success_count, wl->sched_info.workload_id);
    return TT_SUCCESS;
}
