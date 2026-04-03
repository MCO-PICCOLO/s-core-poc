#define _GNU_SOURCE		// required for accept4()
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <stdbool.h>		// true, false

// socket
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>		// inet_ntop()

// systemd
#include <systemd/sd-daemon.h>
#include <systemd/sd-event.h>
#include <systemd/sd-bus.h>

#include "libtrpc.h"
#include "peer_dbus.h"

static const trpc_server_ops_t *trpc_server_ops;

static int trpc_method_register(sd_bus_message *m, void *userdata,
				sd_bus_error *error)
{
	int ret;
	char *name;

	ret = sd_bus_message_read(m, "s", &name);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		return sd_bus_reply_method_error(m, error);
	}

	if (trpc_server_ops && trpc_server_ops->register_cb)
		trpc_server_ops->register_cb(name);

	return sd_bus_reply_method_return(m, NULL);
}

static int trpc_method_schedinfo(sd_bus_message *m, void *userdata,
				sd_bus_error *error)
{
	int ret;
	char *name;
	void *buf = NULL;
	size_t bufsize = 0;
	sd_bus_message *reply_m = NULL;

	ret = sd_bus_message_read(m, "s", &name);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		return sd_bus_reply_method_error(m, error);
	}

	if (trpc_server_ops && trpc_server_ops->schedinfo_cb)
		trpc_server_ops->schedinfo_cb(name, &buf, &bufsize);

	/* can't use sd_bus_reply_method_return() for sending a byte array */
	ret = sd_bus_message_new_method_return(m, &reply_m);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		return sd_bus_reply_method_error(m, error);
	}

	ret = sd_bus_message_append_array(reply_m, 'y', buf, bufsize);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		return sd_bus_reply_method_error(m, error);
	}

#if LIBSYSTEMD_VERSION >= 248
	return sd_bus_message_send(reply_m);
#else
	/* sd_bus_message_send() is not available before v248, use sd_bus_send() instead */
	return sd_bus_send(NULL, reply_m, NULL);
#endif
}

static int trpc_method_dmiss(sd_bus_message *m, void *userdata,
			sd_bus_error *error)
{
	int ret;
	char *name, *task;

	ret = sd_bus_message_read(m, "ss", &name, &task);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		return sd_bus_reply_method_error(m, error);
	}

	if (trpc_server_ops && trpc_server_ops->dmiss_cb)
		trpc_server_ops->dmiss_cb(name, task);

	return sd_bus_reply_method_return(m, NULL);
}

static int trpc_method_sync(sd_bus_message *m, void *userdata,
			sd_bus_error *error)
{
	int ret;
	char *name;
	int ack;
	struct timespec ts;

	ret = sd_bus_message_read(m, "s", &name);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		return sd_bus_reply_method_error(m, error);
	}

	if (trpc_server_ops && trpc_server_ops->sync_cb)
		trpc_server_ops->sync_cb(name, &ack, &ts);

	return sd_bus_reply_method_return(m, "btt", ack, ts.tv_sec, ts.tv_nsec);
}

static const sd_bus_vtable trpc_vtable[] = {
	SD_BUS_VTABLE_START(0),
	SD_BUS_METHOD(TRPC_METHOD_REGISTER, "s", "", trpc_method_register, 0),
	SD_BUS_METHOD(TRPC_METHOD_SCHEDINFO, "s", "ay", trpc_method_schedinfo, 0),
	SD_BUS_METHOD(TRPC_METHOD_DMISS, "ss", "", trpc_method_dmiss, 0),
	SD_BUS_METHOD(TRPC_METHOD_SYNC, "s", "btt", trpc_method_sync, 0),
	SD_BUS_VTABLE_END
};

static int create_server_socket(int port)
{
	int fd;
	int opt;
	struct sockaddr_in saddr;

	fd = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
	if (fd < 0)
		return -errno;

	opt = 1;
	if (setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt)) < 0)
		return -errno;

	saddr.sin_family = AF_INET;
	saddr.sin_addr.s_addr = htonl(INADDR_ANY);
	saddr.sin_port = htons((uint16_t) port);

	if (bind(fd, (struct sockaddr *)&saddr, sizeof(saddr)) < 0)
		return -errno;

	if ((listen(fd, SOMAXCONN)) < 0)
		return -errno;

	return fd;
}

static int set_server_sockopt(int fd)
{
	int ret;
	int optval;

	optval = 1;
	ret = setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &optval, sizeof(optval));
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(errno));
		return -errno;
	}

	optval = 1;
	ret = setsockopt(fd, SOL_SOCKET, SO_KEEPALIVE, &optval, sizeof(optval));
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(errno));
		return -errno;
	}

	optval = 60;
	ret = setsockopt(fd, IPPROTO_TCP, TCP_KEEPIDLE, &optval, sizeof(optval));
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(errno));
		return -errno;
	}

	optval = 10;
	ret = setsockopt(fd, IPPROTO_TCP, TCP_KEEPINTVL, &optval, sizeof(optval));
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(errno));
		return -errno;
	}

	optval = 3;
	ret = setsockopt(fd, IPPROTO_TCP, TCP_KEEPCNT, &optval, sizeof(optval));
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(errno));
		return -errno;
	}

	return 0;
}

static sd_bus *init_dbus_server(sd_event *event, int fd, const char *sender, const char *dbus_desc)
{
	int ret = 0;
	sd_bus *dbus = NULL;
	sd_id128_t server_id;

	ret = sd_id128_randomize(&server_id);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_new(&dbus);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_description(dbus, dbus_desc);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_trusted(dbus, true);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_fd(dbus, fd, fd);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_server(dbus, 1, server_id);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_anonymous(dbus, true);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_negotiate_creds(dbus, 1,
				     SD_BUS_CREDS_PID | SD_BUS_CREDS_UID | SD_BUS_CREDS_EUID |
				     SD_BUS_CREDS_EFFECTIVE_CAPS | SD_BUS_CREDS_SELINUX_CONTEXT);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_sender(dbus, sender);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_start(dbus);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_add_object_vtable(dbus, NULL,
				     TRPC_OBJECT_PATH,
				     TRPC_OBJECT_INTERFACE,
				     trpc_vtable, NULL);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_attach_event(dbus, event, SD_EVENT_PRIORITY_NORMAL);
	if (ret < 0) {
		goto cleanup;
	}

	return dbus;

cleanup:
	LOG_ERROR("%s\n", strerror(-ret));
	if (dbus)
		sd_bus_flush_close_unref(dbus);

	return NULL;
}

#if DEBUG
static void print_client_address(struct sockaddr_in *sa_client)
{
	char buf[INET_ADDRSTRLEN];

	inet_ntop(AF_INET, &sa_client->sin_addr, buf, sizeof(buf));
	LOG_INFO("Connected from %s\n", buf);
}
#else
static inline void print_client_address(struct sockaddr_in *sa_client)
{
}
#endif


static int server_handler(sd_event_source *es, int fd, uint32_t revents, void *userdata)
{
	int connfd;
	sd_bus *dbus;
	sd_event *event = (sd_event *)userdata;
	struct sockaddr_in sa_client;
	socklen_t sa_len;

	bzero(&sa_client, sizeof(sa_client));
	sa_len = sizeof(sa_client);
	connfd = accept4(fd, &sa_client, &sa_len, SOCK_NONBLOCK | SOCK_CLOEXEC);
	if (connfd < 0) {
		if (errno == EAGAIN || errno == EINTR || errno == EWOULDBLOCK) {
			return 0;
		}
		LOG_ERROR("%s\n", strerror(errno));
		return -1;
	}

	print_client_address(&sa_client);

	dbus = init_dbus_server(event, connfd, TRPC_SERVER_NAME, TRPC_SERVER_DESC);
	if (dbus == NULL)
		return -1;

	set_server_sockopt(connfd);

	return 0;
}

int trpc_server_create(int port, sd_event *event,
		sd_event_source **event_source,
		const trpc_server_ops_t *ops)
{
	int fd = -1;
	int ret;
	sd_event_source *source = NULL;

	ret = create_server_socket(port);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}
	fd = ret;

	trpc_server_ops = ops;

	ret = sd_event_add_io(event, &source, fd, EPOLLIN, server_handler, event);
	if (ret < 0)
		goto cleanup;

	if (event_source)
		*event_source = source;

	return fd;

cleanup:
	source = sd_event_source_unref(source);
	if (fd >= 0)
		close(fd);

	return -1;
}

static sd_bus *init_dbus_client(sd_event *event, const char *dbus_desc, const char *serv_addr)
{
	int ret = 0;
	sd_bus *dbus = NULL;

	if (serv_addr == NULL) {
		return NULL;
	}

	ret = sd_bus_new(&dbus);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_description(dbus, dbus_desc);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_trusted(dbus, true);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_set_address(dbus, serv_addr);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_start(dbus);
	if (ret < 0) {
		goto cleanup;
	}

	ret = sd_bus_attach_event(dbus, event, SD_EVENT_PRIORITY_NORMAL);
	if (ret < 0) {
		goto cleanup;
	}

	return dbus;

cleanup:
	LOG_ERROR("%s\n", strerror(-ret));
	if (dbus)
		sd_bus_flush_close_unref(dbus);

	return NULL;
}

int trpc_client_create(const char *serv_addr, sd_event *event, sd_bus **dbus_ret)
{
	int ret;
	sd_bus *dbus;

	dbus = init_dbus_client(event, TRPC_CLIENT_DESC, serv_addr);
	if (dbus == NULL)
		return -1;

	if (dbus_ret)
		*dbus_ret = dbus;

	return 0;
}

int trpc_client_register(sd_bus *dbus, const char *name)
{
	int ret;
	sd_bus_message *reply = NULL;
	sd_bus_error error = SD_BUS_ERROR_NULL;

	ret = sd_bus_call_method(dbus,
				TRPC_SERVER_NAME,
				TRPC_OBJECT_PATH,
				TRPC_OBJECT_INTERFACE,
				TRPC_METHOD_REGISTER,
				&error, &reply,
				"s", name);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}

cleanup:
	sd_bus_error_free(&error);
	if (reply) sd_bus_message_unrefp(&reply);

	return ret;
}

int trpc_client_schedinfo(sd_bus *dbus, const char *name,
			void **buf, size_t *bufsize)
{
	int ret;
	sd_bus_message *reply = NULL;
	sd_bus_error error = SD_BUS_ERROR_NULL;
	const void *r_buf;
	size_t r_bufsize;

	ret = sd_bus_call_method(dbus,
				TRPC_SERVER_NAME,
				TRPC_OBJECT_PATH,
				TRPC_OBJECT_INTERFACE,
				TRPC_METHOD_SCHEDINFO,
				&error, &reply,
				"s", name);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}

	ret = sd_bus_message_read_array(reply, 'y', &r_buf, &r_bufsize);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}

	if (buf && bufsize) {
		*buf = malloc(r_bufsize);
		if (*buf == NULL) {
			LOG_ERROR("Out of memory\n");
			ret = -1;
			goto cleanup;
		}
		memcpy(*buf, r_buf, r_bufsize);
		*bufsize = r_bufsize;
	}

cleanup:
	sd_bus_error_free(&error);
	if (reply) sd_bus_message_unrefp(&reply);

	return ret;
}

int trpc_client_dmiss(sd_bus *dbus, const char *name, const char *task)
{
	int ret;
	sd_bus_message *reply = NULL;
	sd_bus_error error = SD_BUS_ERROR_NULL;

	ret = sd_bus_call_method(dbus,
				TRPC_SERVER_NAME,
				TRPC_OBJECT_PATH,
				TRPC_OBJECT_INTERFACE,
				TRPC_METHOD_DMISS,
				&error, &reply,
				"ss", name, task);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}

cleanup:
	sd_bus_error_free(&error);
	if (reply) sd_bus_message_unrefp(&reply);

	return ret;
}

int trpc_client_sync(sd_bus *dbus, const char *name, int *ack, struct timespec *ts)
{
	int ret;
	sd_bus_message *reply = NULL;
	sd_bus_error error = SD_BUS_ERROR_NULL;

	ret = sd_bus_call_method(dbus,
				TRPC_SERVER_NAME,
				TRPC_OBJECT_PATH,
				TRPC_OBJECT_INTERFACE,
				TRPC_METHOD_SYNC,
				&error, &reply,
				"s", name);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}

	ret = sd_bus_message_read(reply, "btt", ack, &ts->tv_sec, &ts->tv_nsec);
	if (ret < 0) {
		LOG_ERROR("%s\n", strerror(-ret));
		goto cleanup;
	}

cleanup:
	sd_bus_error_free(&error);
	if (reply) sd_bus_message_unrefp(&reply);

	return ret;
}
