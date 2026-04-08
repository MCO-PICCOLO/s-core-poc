/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include "internal.h"

static int init_trpc_connection(const char *addr, int port, sd_bus **dbus_ret, sd_event **event_ret)
{
    int ret;
    char serv_addr[128];

    ret = sd_event_default(event_ret);
    if (ret < 0) {
        return ret;
    }

    snprintf(serv_addr, sizeof(serv_addr), "tcp:host=%s,port=%u", addr, port);
    ret = trpc_client_create(serv_addr, *event_ret, dbus_ret);
    if (ret < 0) {
        *event_ret = sd_event_unref(*event_ret);
        return ret;
    }

    return ret;
}

static void cleanup_trpc_connection(sd_bus **dbus, sd_event **event)
{
    if (*dbus) {
	sd_bus_flush_close_unref(*dbus);
	*dbus = NULL;
    }
    if (*event) {
	sd_event_unref(*event);
	*event = NULL;
    }
}

static int get_sched_info(struct context *ctx, struct sched_info *sinfo)
{
    int ret;
    void *buf = NULL;
    size_t bufsize;
    serial_buf_t *sbuf = NULL;

    ret = trpc_client_schedinfo(ctx->comm.dbus, ctx->config.node_id, &buf, &bufsize);
    if (ret < 0) {
        return -1;
    }

    sbuf = make_serial_buf((void *)buf, bufsize);
    if (sbuf == NULL) {
        return -1;
    }
    buf = NULL;  // now use sbuf->data

    ret = deserialize_sched_info(ctx, sbuf, sinfo);

    free_serial_buf(sbuf);

    return ret;
}

tt_error_t deserialize_sched_info(struct context *ctx, serial_buf_t *sbuf, struct sched_info *sinfo)
{
    uint32_t i;
    uint64_t hyperperiod_us = 0;

    // Unpack sched_info
    if (deserialize_int32_t(sbuf, &sinfo->nr_tasks) < 0) {
        TT_LOG_ERROR("Failed to deserialize nr_tasks");
        return TT_ERROR_NETWORK;
    }
    sinfo->tasks = NULL;

    // Unpack task_info list entries
    for (i = 0; i < sinfo->nr_tasks; i++) {
        struct task_info *tinfo;
        TT_MALLOC(tinfo, struct task_info);

        if (deserialize_str(sbuf, tinfo->node_id) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->allowable_deadline_misses) < 0 ||
            deserialize_int64_t(sbuf, &tinfo->cpu_affinity) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->deadline) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->runtime) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->release_time) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->period) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->sched_policy) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->sched_priority) < 0 ||
            deserialize_str(sbuf, tinfo->name) < 0) {
            TT_LOG_ERROR("Failed to deserialize task_info fields");
            TT_FREE(tinfo);
            destroy_task_info_list(sinfo->tasks);
            sinfo->tasks = NULL;
            return TT_ERROR_NETWORK;
        }

        tinfo->next = sinfo->tasks;
        sinfo->tasks = tinfo;

        TT_LOG_INFO("Task info - name: %s, priority: %d, policy: %d, period: %d",
               tinfo->name, tinfo->sched_priority, tinfo->sched_policy, tinfo->period);
        TT_LOG_INFO("  release_time: %d, runtime: %d, deadline: %d",
               tinfo->release_time, tinfo->runtime, tinfo->deadline);
        TT_LOG_INFO("  cpu_affinity: 0x%lx, allowable_deadline_misses: %d, node_id: %s",
               tinfo->cpu_affinity, tinfo->allowable_deadline_misses, tinfo->node_id);
    }

    if (deserialize_str(sbuf, sinfo->workload_id) < 0 ||
        deserialize_int64_t(sbuf, &hyperperiod_us) < 0) {
        TT_LOG_ERROR("Failed to deserialize workload info");
        destroy_task_info_list(sinfo->tasks);
        sinfo->tasks = NULL;
        return TT_ERROR_NETWORK;
    }

    TT_LOG_INFO("Workload: %s", sinfo->workload_id);
    TT_LOG_INFO("Hyperperiod: %lu us", hyperperiod_us);

    // context의 hp_manager에 초기화 (수정된 부분)
    if (init_hyperperiod(ctx, sinfo->workload_id, hyperperiod_us, &ctx->hp_manager) != TT_SUCCESS) {
        TT_LOG_ERROR("Failed to initialize hyperperiod manager");
        destroy_task_info_list(sinfo->tasks);
        sinfo->tasks = NULL;
        return TT_ERROR_CONFIG;
    }

    return TT_SUCCESS;
}

static int sync_timer_internal(sd_bus *dbus, char *node_id, struct timespec *ts_ptr)
{
    int ret;
    int ack;

    TT_LOG_INFO("Sync");
    fflush(stdout);
    while (1) {
        ret = trpc_client_sync(dbus, node_id, &ack, ts_ptr);
        if (ret < 0) {
            return ret;
        }

        if (ack) {
            TT_LOG_INFO("timestamp: %ld sec %ld nsec", ts_ptr->tv_sec, ts_ptr->tv_nsec);
            break;
        }

        printf(".");
        fflush(stdout);
        /* sleep 100ms to prevent busy polling */
        usleep(TT_POLLING_INTERVAL_US);
    }

    return 0;
}

tt_error_t init_trpc(struct context *ctx)
{
    int retry_count = 0;

    pthread_mutex_init(&ctx->comm.dbus_mutex, NULL);

    // Initialize trpc channel and get schedule info with retry logic
    while (retry_count < TT_MAX_CONNECTION_RETRIES) {
        if (init_trpc_connection(ctx->config.addr, ctx->config.port,
                                &ctx->comm.dbus, &ctx->comm.event) == 0) {
            if (ctx->config.enable_apex) {
		TT_LOG_INFO("Apex.OS test mode enabled, proceeding without schedule info");
		return TT_SUCCESS;
	    }

            if (get_sched_info(ctx, &ctx->runtime.sched_info) == 0) {
                /* Successfully retrieved schedule info */
                TT_LOG_INFO("Successfully connected and retrieved schedule info (attempt %d)", retry_count + 1);
                return TT_SUCCESS;
            }
	    // Failed to get schedule info, clean up the connection resources
	    cleanup_trpc_connection(&ctx->comm.dbus, &ctx->comm.event);
        }

        /* failed to get schedule info, retrying */
        retry_count++;
        TT_LOG_INFO("Connection attempt %d/%d failed, retrying...", retry_count, TT_MAX_CONNECTION_RETRIES);
        usleep(TT_RETRY_INTERVAL_US);
    }

    TT_LOG_ERROR("Failed to connect to server after %d attempts", TT_MAX_CONNECTION_RETRIES);
    return TT_ERROR_NETWORK;
}

/*
 * Fetch the current schedule from timpani-o WITHOUT touching hyperperiod
 * state (i.e. does not call init_hyperperiod).  Used by the periodic
 * affinity-refresh path.  Caller must call destroy_task_info_list() on
 * sinfo->tasks when done.
 */
tt_error_t poll_sched_info(struct context *ctx, struct sched_info *sinfo)
{
    void *buf = NULL;
    size_t bufsize;
    serial_buf_t *sbuf;
    uint32_t i;
    char workload_id[64];
    uint64_t hyperperiod_us = 0;

    pthread_mutex_lock(&ctx->comm.dbus_mutex);
    if (trpc_client_schedinfo(ctx->comm.dbus, ctx->config.node_id,
                              &buf, &bufsize) < 0) {
        pthread_mutex_unlock(&ctx->comm.dbus_mutex);
        return TT_ERROR_NETWORK;
    }
    pthread_mutex_unlock(&ctx->comm.dbus_mutex);

    if (!buf || bufsize == 0) {
        TT_LOG_ERROR("Empty schedule info response during poll");
        free(buf);
        return TT_ERROR_NETWORK;
    }

    sbuf = make_serial_buf(buf, bufsize);
    if (!sbuf) { free(buf); return TT_ERROR_MEMORY; }
    buf = NULL;

    memset(sinfo, 0, sizeof(*sinfo));

    if (deserialize_int32_t(sbuf, &sinfo->nr_tasks) < 0) {
        TT_LOG_ERROR("poll_sched_info: failed to deserialize nr_tasks");
        free_serial_buf(sbuf);
        return TT_ERROR_NETWORK;
    }

    for (i = 0; i < sinfo->nr_tasks; i++) {
        struct task_info *tinfo;
        TT_MALLOC(tinfo, struct task_info);

        if (deserialize_str(sbuf, tinfo->node_id) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->allowable_deadline_misses) < 0 ||
            deserialize_int64_t(sbuf, &tinfo->cpu_affinity) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->deadline) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->runtime) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->release_time) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->period) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->sched_policy) < 0 ||
            deserialize_int32_t(sbuf, &tinfo->sched_priority) < 0 ||
            deserialize_str(sbuf, tinfo->name) < 0) {
            TT_LOG_ERROR("poll_sched_info: failed to deserialize task %u", i);
            TT_FREE(tinfo);
            destroy_task_info_list(sinfo->tasks);
            sinfo->tasks = NULL;
            free_serial_buf(sbuf);
            return TT_ERROR_NETWORK;
        }

        tinfo->next = sinfo->tasks;
        sinfo->tasks = tinfo;
    }

    /* Read trailing workload_id + hyperperiod and store them in sinfo so that
     * callers (e.g. register_new_tasks) can tell which workload the tasks
     * belong to and what hyperperiod length it requires. */
    if (deserialize_str(sbuf, workload_id) < 0 ||
        deserialize_int64_t(sbuf, &hyperperiod_us) < 0) {
        TT_LOG_ERROR("poll_sched_info: failed to deserialize workload info");
        destroy_task_info_list(sinfo->tasks);
        sinfo->tasks = NULL;
        free_serial_buf(sbuf);
        return TT_ERROR_NETWORK;
    }

    /* Populate the sinfo fields so callers know which workload this is and
     * what hyperperiod it uses — previously these were discarded here. */
    strncpy(sinfo->workload_id, workload_id, sizeof(sinfo->workload_id) - 1);
    sinfo->workload_id[sizeof(sinfo->workload_id) - 1] = '\0';
    sinfo->pod_period = (int32_t)hyperperiod_us;   /* reuse pod_period to carry hyperperiod_us */

    TT_LOG_DEBUG("poll_sched_info: workload_id='%s' hyperperiod=%lu us",
                 sinfo->workload_id, hyperperiod_us);

    free_serial_buf(sbuf);
    return TT_SUCCESS;
}

tt_error_t sync_timer_with_server(struct context *ctx)
{
    if (!ctx->config.enable_sync) {
        return TT_SUCCESS;
    }

    if (sync_timer_internal(ctx->comm.dbus, ctx->config.node_id,
                           &ctx->runtime.starttimer_ts) < 0) {
        return TT_ERROR_NETWORK;
    }

    return TT_SUCCESS;
}

tt_error_t report_deadline_miss(struct context *ctx, const char *taskname)
{
    pthread_mutex_lock(&ctx->comm.dbus_mutex);
    int result = trpc_client_dmiss(ctx->comm.dbus, ctx->config.node_id, taskname);
    pthread_mutex_unlock(&ctx->comm.dbus_mutex);
    return (result < 0) ? TT_ERROR_NETWORK : TT_SUCCESS;
}
