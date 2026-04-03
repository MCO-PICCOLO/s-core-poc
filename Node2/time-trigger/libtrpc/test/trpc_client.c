#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <assert.h>
#include <time.h>

#include <libtrpc.h>

#include "schedinfo.h"

#define CLIENT_NAME	"Timpani-N"
#define SERVER_IPADDR	"localhost"
#define SERVER_PORT	7777

static struct sched_info sched_info;

static int register_to_server(sd_bus *dbus)
{
	int ret;

	ret = trpc_client_register(dbus, CLIENT_NAME);
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
	}
	return ret;
}

static void deserialize_schedinfo(serial_buf_t *sbuf, struct sched_info *sinfo)
{
	uint32_t i;
	uint32_t cid_size;

	// Unpack sched_info
	deserialize_int32_t(sbuf, &sinfo->nr_tasks);
	deserialize_int32_t(sbuf, &sinfo->pod_period);
	deserialize_int32_t(sbuf, &sinfo->container_period);
	deserialize_int64_t(sbuf, &sinfo->cpumask);
	deserialize_int32_t(sbuf, &sinfo->container_rt_period);
	deserialize_int32_t(sbuf, &sinfo->container_rt_runtime);
	cid_size = sizeof(sinfo->container_id);
	deserialize_blob(sbuf, sinfo->container_id, &cid_size);

	sinfo->tasks = NULL;

	printf("sinfo->container_id: %.*s\n", cid_size, sinfo->container_id);
	printf("sinfo->container_rt_runtime: %u\n", sinfo->container_rt_runtime);
	printf("sinfo->container_rt_period: %u\n", sinfo->container_rt_period);
	printf("sinfo->cpumask: %"PRIx64"\n", sinfo->cpumask);
	printf("sinfo->container_period: %u\n", sinfo->container_period);
	printf("sinfo->pod_period: %u\n", sinfo->pod_period);
	printf("sinfo->nr_tasks: %u\n", sinfo->nr_tasks);


	// Unpack task_info list entries
	for (i = 0; i < sinfo->nr_tasks; i++) {
		struct task_info *tinfo = malloc(sizeof(struct task_info));
		assert(tinfo);

		deserialize_int32_t(sbuf, &tinfo->release_time);
		deserialize_int32_t(sbuf, &tinfo->period);
		deserialize_str(sbuf, tinfo->name);
		deserialize_int32_t(sbuf, &tinfo->pid);

		tinfo->next = sinfo->tasks;
		sinfo->tasks = tinfo;

		printf("tinfo->pid: %u\n", tinfo->pid);
		printf("tinfo->name: %s\n", tinfo->name);
		printf("tinfo->period: %d\n", tinfo->period);
		printf("tinfo->release_time: %d\n", tinfo->release_time);
	}
}


static int get_schedinfo(sd_bus *dbus)
{
	int ret;
	void *buf = NULL;
	size_t bufsize;
	serial_buf_t *sbuf = NULL;
	uint64_t u64;
	char str[64];

	ret = trpc_client_schedinfo(dbus, CLIENT_NAME, &buf, &bufsize);
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
		return ret;
	}

	printf("Received %zu bytes\n", bufsize);

	sbuf = make_serial_buf((void *)buf, bufsize);
	if (sbuf == NULL) {
		fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
		return -1;
	}
	buf = NULL;	// now use sbuf->data

	deserialize_schedinfo(sbuf, &sched_info);

	free_serial_buf(sbuf);

	return 0;
}

static int report_dmiss(sd_bus *dbus)
{
	int ret;

	ret = trpc_client_dmiss(dbus, CLIENT_NAME, "hello1");
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
	}
	return ret;
}

static void timespec_to_str(struct timespec *ts, char *str, int len)
{
	int ret;
	struct tm tm;

	gmtime_r(&ts->tv_sec, &tm);

	ret = strftime(str, len, "%F %T", &tm);
	len -= ret - 1;

	snprintf(str + strlen(str), len, ".%09ld", ts->tv_nsec);
}

static int wait_for_sync(sd_bus *dbus)
{
	int ret;
	int ack;
	struct timespec ts;

	while (1) {
		ret = trpc_client_sync(dbus, CLIENT_NAME, &ack, &ts);
		if (ret < 0) {
			fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
			return ret;
		}

		if (ack) {
			char str[64];

			timespec_to_str(&ts, str, sizeof(str));
			printf("Sync time: %s (%ld:%ld)\n", str, ts.tv_sec, ts.tv_nsec);
			break;
		}

		printf("got NACK !\n");
		usleep(500000);
	}

	return 0;
}

int main(int argc, char *argv[])
{
	sd_event *event = NULL;
	sd_bus *dbus = NULL;
	int fd = -1;
	int ret;
	char *addr = SERVER_IPADDR;
	uint32_t port = SERVER_PORT;
	char serv_addr[128];

	if (argc > 1) {
		addr = argv[1];
		if (argc > 2) {
			 port = atoi(argv[2]);
		}
	}

	ret = sd_event_default(&event);
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
		goto out;
	}

	snprintf(serv_addr, sizeof(serv_addr), "tcp:host=%s,port=%u", addr, port);
	ret = trpc_client_create(serv_addr, event, &dbus);
	if (ret < 0) {
		fprintf(stderr, "%s:%d: %s\n", __func__, __LINE__, strerror(-ret));
		goto out;
	}
	fd = ret;

	ret = register_to_server(dbus);
	if (ret < 0) {
		goto out;
	}

	ret = get_schedinfo(dbus);
	if (ret < 0) {
		goto out;
	}

	ret = wait_for_sync(dbus);
	if (ret < 0) {
		goto out;
	}

	ret = report_dmiss(dbus);
	if (ret < 0) {
		goto out;
	}

	ret = sd_event_loop(event);
	if (ret < 0)
		goto out;

out:
	event = sd_event_unref(event);
	if (fd >= 0)
		close(fd);
	return ret < 0 ? EXIT_FAILURE : EXIT_SUCCESS;
}
