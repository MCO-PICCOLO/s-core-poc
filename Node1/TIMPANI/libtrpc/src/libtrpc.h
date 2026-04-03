/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef _LIBTRPC_H_
#define _LIBTRPC_H_

#ifdef __cplusplus
extern "C" {
#endif

/*
 * D-Bus stuff
 */

#include <systemd/sd-event.h>
#include <systemd/sd-bus.h>

typedef struct {
	void (*register_cb)(const char *name);
	void (*schedinfo_cb)(const char *name, void **buf, size_t *bufsize);
	void (*dmiss_cb)(const char *name, const char *task);
	void (*sync_cb)(const char *name, int *ack, struct timespec *ts);
} trpc_server_ops_t;

int trpc_server_create(int port, sd_event *event,
		sd_event_source **event_source,
		const trpc_server_ops_t *ops);
int trpc_client_create(const char *serv_addr, sd_event *event,
		sd_bus **dbus_ret);
int trpc_client_register(sd_bus *dbus, const char *name);
int trpc_client_schedinfo(sd_bus *dbus, const char *name,
			void **buf, size_t *bufsize);
int trpc_client_dmiss(sd_bus *dbus, const char *name, const char *task);
int trpc_client_sync(sd_bus *dbus, const char *name, int *ack, struct timespec *ts);

/*
 * Data serialization stuff
 */

#define DECLARE_SERIALIZE(_type)					\
	int serialize_##_type(serial_buf_t *b, _type t);		\
	int deserialize_##_type(serial_buf_t *b, _type *t);

typedef struct {
	void *data;
	size_t pos;
	size_t size;
} serial_buf_t;

serial_buf_t *new_serial_buf(size_t size);
serial_buf_t *make_serial_buf(void *data, size_t size);
void free_serial_buf(serial_buf_t *b);
void reset_serial_buf(serial_buf_t *b);

DECLARE_SERIALIZE(int8_t)
DECLARE_SERIALIZE(int16_t)
DECLARE_SERIALIZE(int32_t)
DECLARE_SERIALIZE(int64_t)
DECLARE_SERIALIZE(float)
DECLARE_SERIALIZE(double)

int serialize_blob(serial_buf_t *b, const char *t, uint32_t bytes);
int deserialize_blob(serial_buf_t *b, char *t, uint32_t *bytes);
int serialize_str(serial_buf_t *b, const char *t);
int deserialize_str(serial_buf_t *b, char *t);

#ifdef __cplusplus
}
#endif

#endif	/* _LIBTRPC_H_ */
