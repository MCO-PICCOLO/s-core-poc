/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <assert.h>
#include <time.h>

#include <libtrpc.h>

#include "schedinfo.h"

#define SERVER_PORT	7777
#define CONTAINER_ID	"cc5c0d4ba8e10568df25f67b6f396da65c2615a4e6dd6f2f0ad554e9465fbb55"

static serial_buf_t *sbuf;
static struct sched_info sched_info;

static void serialize_schedinfo(struct sched_info *sinfo)
{
	uint32_t nr_tasks = 0;

	if (sbuf) return;

	sbuf = new_serial_buf(256);
	assert(sbuf);

	printf("sinfo->nr_tasks: %u\n", sinfo->nr_tasks);

	// Pack task_info list entries into serial buffer.
	for (struct task_info *t = sinfo->tasks; t; t = t->next) {
		printf("t->pid: %u\n", t->pid);
		printf("t->name: %s\n", t->name);
		printf("t->period: %u\n", t->period);
		printf("t->release_time: %u\n", t->release_time);

		serialize_int32_t(sbuf, t->pid);
		serialize_str(sbuf, t->name);
		serialize_int32_t(sbuf, t->period);
		serialize_int32_t(sbuf, t->release_time);
		nr_tasks++;
	}

	if (nr_tasks != sinfo->nr_tasks) {
		printf("WARNING: counted nr_tasks(%u) is different from sched_info->nr_task(%u)\n",
			nr_tasks, sinfo->nr_tasks);
	}

	// Pack sched_info into serial buffer.
	serialize_blob(sbuf, sinfo->container_id, sizeof(sinfo->container_id));
	serialize_int32_t(sbuf, sinfo->container_rt_runtime);
	serialize_int32_t(sbuf, sinfo->container_rt_period);
	serialize_int64_t(sbuf, sinfo->cpumask);
	serialize_int32_t(sbuf, sinfo->container_period);
	serialize_int32_t(sbuf, sinfo->pod_period);
	serialize_int32_t(sbuf, sinfo->nr_tasks);
}

static void register_callback(const char *name)
{
	printf("Register: %s\n", name);
}

static void dmiss_callback(const char *name, const char *task)
{
	printf("Deadline miss: %s @ %s\n", task, name);
}

static void sync_callback(const char *name, int *ack, struct timespec *ts)
{
	static int sync_count;

	printf("Sync from %s\n", name);
	if (sync_count++ < 2) {
		printf("Send NACK to %s\n", name);
		*ack = 0;
	} else {
		printf("Send ACK to %s\n", name);
		*ack = 1;
		clock_gettime(CLOCK_REALTIME, ts);
	}
}

static void schedinfo_callback(const char *name, void **buf, size_t *bufsize)
{
	printf("SchedInfo: %s\n", name);

	serialize_schedinfo(&sched_info);

	printf("sbuf size: %zu\n", sbuf->pos);

	if (buf)
		*buf = sbuf->data;
	if (bufsize)
		*bufsize = sbuf->pos;
}

static void init_schedinfo(struct sched_info *sinfo)
{
	struct task_info *tinfo;
	int i;

	memcpy(sinfo->container_id, CONTAINER_ID, sizeof(CONTAINER_ID) - 1);
	sinfo->container_rt_runtime = 800000;
	sinfo->container_rt_period = 1000000;
	sinfo->cpumask = 0xffffffff;
	sinfo->container_period = 1000000;
	sinfo->pod_period = 1000000;
	sinfo->nr_tasks = 0;
	sinfo->tasks = NULL;

	for (i = 1; i <= 4; i++) {
		tinfo = malloc(sizeof(struct task_info));
		assert(tinfo);

		tinfo->pid = i;
		snprintf(tinfo->name, sizeof(tinfo->name), "%s%d", "hello", i);
		tinfo->period = 20000 * i;
		tinfo->release_time = i;

		tinfo->next = sinfo->tasks;
		sinfo->tasks = tinfo;
		sinfo->nr_tasks++;
	}
}

int main(int argc, char *argv[])
{
	sd_event_source *event_source = NULL;
	sd_event *event = NULL;
	int fd = -1;
	int ret;
	trpc_server_ops_t ops = {
		.register_cb = register_callback,
		.schedinfo_cb = schedinfo_callback,
		.dmiss_cb = dmiss_callback,
		.sync_cb = sync_callback,
	};
	uint32_t port = SERVER_PORT;

	if (argc > 1) {
		port = atoi(argv[1]);
	}

	init_schedinfo(&sched_info);

	ret = sd_event_default(&event);
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __FUNCTION__, __LINE__, strerror(-ret));
		goto out;
	}

	ret = trpc_server_create(port, event, &event_source, &ops);
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __FUNCTION__, __LINE__, strerror(-ret));
		goto out;
	}
	fd = ret;
	printf("Listening on %u...\n", port);

	ret = sd_event_loop(event);
	if (ret < 0)
		goto out;

out:
	event_source = sd_event_source_unref(event_source);
	event = sd_event_unref(event);
	if (fd >= 0)
		close(fd);
	return ret < 0 ? EXIT_FAILURE : EXIT_SUCCESS;
}
