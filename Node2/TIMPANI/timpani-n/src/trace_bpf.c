/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <limits.h>

#include <pthread.h>

#include <bpf/libbpf.h>
#include <bpf/bpf.h>

#include "trace_bpf.h"

#include "sigwait.skel.h"
#ifdef CONFIG_TRACE_BPF_EVENT
#include "schedstat.skel.h"
#endif

#include "internal.h"

#define RB_TIMEOUT_MS		100

typedef struct {
	struct ring_buffer *rb;
	pthread_t thread;
	int need_exit;
} rb_data_t;

static struct sigwait_bpf *sigwait_bpf;
static rb_data_t sigwait_rb_data;

static struct schedstat_bpf *schedstat_bpf;
static rb_data_t schedstat_rb_data;

static int libbpf_print_fn(enum libbpf_print_level level, const char *format,
			   va_list args)
{
	if (level == LIBBPF_DEBUG) return 0;
	return vfprintf(stderr, format, args);
}

static void *rb_thread_func(void *arg)
{
	int ret;
	rb_data_t *rb_data = (rb_data_t *)arg;

	while (!rb_data->need_exit) {
		ret = ring_buffer__poll(rb_data->rb, RB_TIMEOUT_MS);
		if (ret < 0 && ret != -EINTR) {
			fprintf(stderr, "Error ring_buffer__poll: %d\n", ret);
			break;
		}
	}
	pthread_exit(NULL);
	return NULL;
}

static int start_sigwait_bpf(ring_buffer_sample_fn sigwait_cb, void *ctx)
{
	int ret;

	sigwait_bpf = sigwait_bpf__open();
	if (!sigwait_bpf) {
		fprintf(stderr, "Error bpf__open\n");
		goto fail;
	}

	ret = sigwait_bpf__load(sigwait_bpf);
	if (ret < 0) {
		fprintf(stderr, "Error bpf__load\n");
		goto fail;
	}

	sigwait_rb_data.rb = ring_buffer__new(bpf_map__fd(sigwait_bpf->maps.buffer), sigwait_cb, ctx, NULL);
	if (!sigwait_rb_data.rb) {
		fprintf(stderr, "Error creating ring buffer\n");
		goto fail;
	}

	ret = sigwait_bpf__attach(sigwait_bpf);
	if (ret < 0) {
		fprintf(stderr, "Error bpf__attach\n");
		goto fail;
	}

	if (pthread_create(&sigwait_rb_data.thread, NULL, rb_thread_func, (void *)&sigwait_rb_data)) {
		fprintf(stderr, "Error pthread_create\n");
		goto fail;
	}

	return 0;

fail:
	if (sigwait_rb_data.rb) {
		ring_buffer__free(sigwait_rb_data.rb);
		sigwait_rb_data.rb = NULL;
	}
	if (sigwait_bpf) {
		sigwait_bpf__destroy(sigwait_bpf);
		sigwait_bpf = NULL;
	}
	return ret;
}

#ifdef CONFIG_TRACE_BPF_EVENT
static int start_schedstat_bpf(ring_buffer_sample_fn schedstat_cb, void *ctx)
{
	int ret;

	schedstat_bpf = schedstat_bpf__open();
	if (!schedstat_bpf) {
		fprintf(stderr, "Error bpf__open\n");
		goto fail;
	}

	ret = schedstat_bpf__load(schedstat_bpf);
	if (ret < 0) {
		fprintf(stderr, "Error bpf__load\n");
		goto fail;
	}

	schedstat_rb_data.rb = ring_buffer__new(bpf_map__fd(schedstat_bpf->maps.buffer), schedstat_cb, ctx, NULL);
	if (!schedstat_rb_data.rb) {
		fprintf(stderr, "Error creating ring buffer\n");
		goto fail;
	}

	ret = schedstat_bpf__attach(schedstat_bpf);
	if (ret < 0) {
		fprintf(stderr, "Error bpf__attach\n");
		goto fail;
	}

	if (pthread_create(&schedstat_rb_data.thread, NULL, rb_thread_func, (void *)&schedstat_rb_data)) {
		fprintf(stderr, "Error pthread_create\n");
		goto fail;
	}

	return 0;

fail:
	if (schedstat_rb_data.rb) {
		ring_buffer__free(schedstat_rb_data.rb);
		schedstat_rb_data.rb = NULL;
	}
	if (schedstat_bpf) {
		schedstat_bpf__destroy(schedstat_bpf);
		schedstat_bpf = NULL;
	}
	return ret;
}
#endif /* CONFIG_TRACE_BPF_EVENT */


int bpf_on(ring_buffer_sample_fn sigwait_cb,
	ring_buffer_sample_fn schedstat_cb,
	void *ctx)
{
	int ret;

	libbpf_set_print(libbpf_print_fn);

	ret = start_sigwait_bpf(sigwait_cb, ctx);
	if (ret < 0) {
		fprintf(stderr, "Warning: Failed to initialize BPF tracepoints - continuing without BPF monitoring\n");
		fprintf(stderr, "Info: This is normal on kernels without required tracepoint support\n");
		return 0;  // Return success to allow application to continue
	}

#ifdef CONFIG_TRACE_BPF_EVENT
	ret = start_schedstat_bpf(schedstat_cb, ctx);
	if (ret < 0) {
		fprintf(stderr, "Warning: Failed to initialize BPF schedstat monitoring - continuing without schedstat\n");
		// Don't fail completely, sigwait BPF is more important
	}
#endif

	return 0;  // Always return success for graceful degradation
}

void bpf_off(void)
{
	if (sigwait_bpf) {
		sigwait_rb_data.need_exit = 1;
		pthread_join(sigwait_rb_data.thread, NULL);

		if (sigwait_rb_data.rb) {
			ring_buffer__free(sigwait_rb_data.rb);
			sigwait_rb_data.rb = NULL;
		}

		sigwait_bpf__destroy(sigwait_bpf);
		sigwait_bpf = NULL;
	}

#ifdef CONFIG_TRACE_BPF_EVENT
	if (schedstat_bpf) {
		schedstat_rb_data.need_exit = 1;
		pthread_join(schedstat_rb_data.thread, NULL);

		if (schedstat_rb_data.rb) {
			ring_buffer__free(schedstat_rb_data.rb);
			schedstat_rb_data.rb = NULL;
		}

		schedstat_bpf__destroy(schedstat_bpf);
		schedstat_bpf = NULL;
	}
#endif
}

int bpf_add_pid(int pid)
{
	uint8_t value = 1;

	// Check if sigwait BPF feature is initialized
	if (!sigwait_bpf) {
		// BPF not available, return success for graceful degradation
		return 0;
	}

	if (bpf_map_update_elem(bpf_map__fd(sigwait_bpf->maps.pid_filter_map), &pid, &value, BPF_ANY)) {
		fprintf(stderr, "Warning: Failed to add PID %d to BPF pid_filter_map\n", pid);
		return 0;  // Don't fail the application
	}

#ifdef CONFIG_TRACE_BPF_EVENT
	// Check if schedstat BPF feature is initialized
	if (schedstat_bpf) {
		if (bpf_map_update_elem(bpf_map__fd(schedstat_bpf->maps.pid_filter_map), &pid, &value, BPF_ANY)) {
			fprintf(stderr, "Warning: Failed to add PID %d to schedstat pid_filter_map\n", pid);
		}
	}
#endif

	return 0;
}

int bpf_del_pid(int pid)
{
	// Check if sigwait BPF feature is initialized
	if (!sigwait_bpf) {
		// BPF not available, return success for graceful degradation
		return 0;
	}

	if (bpf_map_delete_elem(bpf_map__fd(sigwait_bpf->maps.pid_filter_map), &pid)) {
		fprintf(stderr, "Warning: Failed to delete PID %d from BPF pid_filter_map\n", pid);
		return 0;  // Don't fail the application
	}

#ifdef CONFIG_TRACE_BPF_EVENT
	// Check if schedstat BPF feature is initialized
	if (schedstat_bpf) {
		if (bpf_map_delete_elem(bpf_map__fd(schedstat_bpf->maps.pid_filter_map), &pid)) {
			fprintf(stderr, "Warning: Failed to delete PID %d from schedstat pid_filter_map\n", pid);
		}
	}
#endif

	return 0;
}
