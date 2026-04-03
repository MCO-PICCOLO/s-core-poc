/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <unistd.h>
#include <errno.h>
#include <stdatomic.h>

#include "internal.h"

// Apex.OS UDS path definitions
#define SOCKET_DIR "/tmp/timpani/"
#define SOCKET_FILE "timpani.sock"
#define SOCKET_PATH SOCKET_DIR SOCKET_FILE

// Communication message structure for Apex.OS
// Refer to internal.h for enum definitions
typedef struct {
  int msg_type;
  union {
    struct {
      char name[MAX_APEX_NAME_LEN];
      int type;
    } fault;
    struct {
      char name[MAX_APEX_NAME_LEN];
      int pid;
    } up;
    struct {
      int pid;
    } down;
  } data;
} timpani_msg_t;

// Initialize Apex.OS Monitor UDS
int apex_monitor_init(struct context *ctx)
{
	int server_fd;
	struct sockaddr_un server_addr;

	// Create Unix domain socket
	server_fd = socket(AF_UNIX, SOCK_DGRAM, 0);
	if (server_fd == -1) {
		return TT_ERROR_NETWORK;
	}

	TT_LOG_INFO("Apex.OS Monitor socket created: %s", SOCKET_PATH);

	// Remove any existing socket file
	if (access(SOCKET_PATH, F_OK) == 0) {
		unlink(SOCKET_PATH);
	} else {
		// Ensure the directory exists
		mkdir(SOCKET_DIR, 0755);
	}

	// Set up server address
	memset(&server_addr, 0, sizeof(server_addr));
	server_addr.sun_family = AF_UNIX;
	strncpy(server_addr.sun_path, SOCKET_PATH,
		sizeof(server_addr.sun_path) - 1);

	// Bind socket to address
	if (bind(server_fd, (struct sockaddr *)&server_addr,
		 sizeof(server_addr)) < 0) {
		close(server_fd);
		return TT_ERROR_NETWORK;
	}

	// Set socket permissions to allow write for all users
	if (chmod(SOCKET_PATH, S_IRUSR | S_IWUSR | S_IWGRP | S_IWOTH) < 0) {
		close(server_fd);
		unlink(SOCKET_PATH);
		return TT_ERROR_NETWORK;
	}

	ctx->comm.apex_fd = server_fd;
	return TT_SUCCESS;
}

// Cleanup Apex.OS Monitor UDS
void apex_monitor_cleanup(struct context *ctx)
{
	int server_fd = ctx->comm.apex_fd;

	if (server_fd != -1) {
		ctx->comm.apex_fd = -1;
		close(server_fd);
		unlink(SOCKET_PATH);
	}
}

// Receive a Apex.OS fault event
int apex_monitor_recv(struct context *ctx, char *name, int size, int *pid, int *type)
{
	int ret;
	int server_fd = ctx->comm.apex_fd;
	timpani_msg_t msg;

	ret = recvfrom(server_fd, &msg, sizeof(msg), 0, NULL, NULL);
	if (ret < 0) {
		if (errno == EAGAIN) {
			// No data available
			return TT_ERROR_IO;
		}
		return TT_ERROR_NETWORK;
	} else if (ret == 0) {
		// No data received
		return TT_ERROR_IO;
	}

	if (msg.msg_type == APEX_FAULT) {
		if (name) {
			strncpy(name, msg.data.fault.name, size - 1);
			name[size - 1] = '\0';
		}
	} else if (msg.msg_type == APEX_UP) {
		if (name) {
			strncpy(name, msg.data.up.name, size - 1);
			name[size - 1] = '\0';
		}
		if (pid) {
			*pid = msg.data.up.pid;
		}
	} else if (msg.msg_type == APEX_DOWN) {
		if (pid) {
			*pid = msg.data.down.pid;
		}
	} else if (msg.msg_type == APEX_RESET) {
		// RESET message received, it is for DEMO purpose only
	} else {
		TT_LOG_WARNING("Unknown Apex.OS message type: %d", msg.msg_type);
		return TT_ERROR_IO;
	}

	if (type) {
		*type = msg.msg_type;
	}
	// Data received
	return TT_SUCCESS;
}

// Initialize Apex.OS task list from sched_info
tt_error_t init_apex_list(struct context *ctx)
{
	int success_count = 0;

	// LIST_INIT is already invoked at config_set_defaults

	for (struct task_info *ti = ctx->runtime.sched_info.tasks; ti; ti = ti->next) {
		if (strcmp(ctx->config.node_id, ti->node_id) != 0) {
			/* The task does not belong to this node. */
			continue;
		}

		struct apex_info *apex_task = calloc(1, sizeof(struct apex_info));
		if (!apex_task) {
			TT_LOG_ERROR("Failed to allocate memory for Apex.OS task");
			continue;
		}
		memcpy(&apex_task->task, ti, sizeof(apex_task->task));

		LIST_INSERT_HEAD(&ctx->runtime.apex_list, apex_task, entry);
		TT_LOG_INFO("Initialized Apex.OS task: %s", ti->name);
		success_count++;
	}

	if (success_count == 0) {
		TT_LOG_ERROR("No tasks were successfully initialized");
		return TT_ERROR_CONFIG;
	}

	TT_LOG_INFO("Successfully initialized %d tasks", success_count);
	return TT_SUCCESS;
}

// CoreData Provider communication definitions and structures
// Copied from http://mod.lge.com/hub/sdvcore/coredata_provider/-/blob/master/src/app_data_receiver.hpp
#define COREDATA_SOCKET_PATH "/tmp/appdata/appdata.sock"

#define MAX_APP_NAME_LEN   256
#define MAX_CORE_MASK_LEN  64
#define MAX_JSON_DATA_LEN  10240

enum {
  APP_STATUS = 0,
  DMISS_STATUS = 1,
};

struct app_data_msg_t {
    int msg_type;
    union {
        struct {
            char name[MAX_APP_NAME_LEN];
            int pid;
            double fps;
            double latency;
            char core_masking[MAX_CORE_MASK_LEN];
            char safety_core_masking[MAX_CORE_MASK_LEN];
        } app_status;

        struct {
            char name[MAX_APP_NAME_LEN];
            int pid;
            int dmiss_max;
            int dmiss_count;
            int64_t period_us;
        } dmiss_status;
    };
};

// Initialize coredata provider client UDS
static int coredata_client_init(void)
{
	int fd;

	// Create Unix domain socket
	fd = socket(AF_UNIX, SOCK_DGRAM, 0);
	if (fd == -1) {
		return -1;
	}

	return fd;
}

// Send deadline miss info to coredata provider
tt_error_t coredata_client_send(struct apex_info *app)
{
	static int coredata_fd = -1;
	static struct sockaddr_un server_addr;
	int ret;
	struct app_data_msg_t msg;

	if (coredata_fd == -1) {
		// init client socket for coredata provider
		ret = coredata_client_init();
		if (ret == -1)
			return TT_ERROR_IO;
		coredata_fd = ret;

		// Set up server address
		memset(&server_addr, 0, sizeof(server_addr));
		server_addr.sun_family = AF_UNIX;
		strncpy(server_addr.sun_path, COREDATA_SOCKET_PATH, sizeof(server_addr.sun_path) - 1);
	}

	// Prepare message
	msg.msg_type = DMISS_STATUS;
	strncpy(msg.dmiss_status.name, app->name, MAX_APP_NAME_LEN - 1);
	msg.dmiss_status.name[MAX_APP_NAME_LEN - 1] = '\0';
	msg.dmiss_status.pid = app->nspid;
	msg.dmiss_status.period_us = app->task.period;
	msg.dmiss_status.dmiss_max = app->task.allowable_deadline_misses;
	msg.dmiss_status.dmiss_count = atomic_load(&app->dmiss_count);

	// Send message to coredata provider
	ret = sendto(coredata_fd, &msg, sizeof(msg), 0,
		(struct sockaddr*)&server_addr, sizeof(server_addr));
        if (ret == -1) {
		return TT_ERROR_IO;
        }

	return TT_SUCCESS;
}

// Timer handler for periodic coredata reporting
static void coredata_timer_handler(union sigval sv)
{
	struct apex_info *app = (struct apex_info *)sv.sival_ptr;
	struct timespec now;
	uint64_t delta;
	uint64_t dmiss_time_us = atomic_load(&app->dmiss_time_us);

	if (dmiss_time_us != 0) {
		clock_gettime(CLOCK_MONOTONIC, &now);
		delta = tt_timespec_to_us(&now) - dmiss_time_us;
		if (delta > app->task.period) {
			// Task period expired, so reset counters
            		atomic_store(&app->dmiss_count, 0);
            		atomic_store(&app->dmiss_time_us, 0);
		}
	}

	coredata_client_send(app);
}

// Create a timer for periodic coredata reporting
tt_error_t coredata_create_timer(struct apex_info *app)
{
	struct sigevent sev;
	struct itimerspec its;

	if (!app) return TT_ERROR_INVALID_ARGS;

	// Configure the sigevent structure
	memset(&sev, 0, sizeof(struct sigevent));
	sev.sigev_notify = SIGEV_THREAD;
	sev.sigev_notify_function = coredata_timer_handler;
	sev.sigev_value.sival_ptr = app;

	// Create the timer
	if (timer_create(CLOCK_REALTIME, &sev, &app->coredata_timer) == -1) {
		TT_LOG_ERROR("Failed to create coredata timer");
		return TT_ERROR_TIMER;
	}

	// Configure the timer to expire immediately at first, then every 0.5 second
	its.it_value.tv_sec = 0;      // Initial expiration: 1 ms
	its.it_value.tv_nsec = 1000;
	its.it_interval.tv_sec = 0;   // Interval: 500 ms
	its.it_interval.tv_nsec = 500000000;

	// Start the timer
	if (timer_settime(app->coredata_timer, 0, &its, NULL) == -1) {
		TT_LOG_ERROR("Failed to start coredata timer");
		timer_delete(app->coredata_timer);
		return TT_ERROR_TIMER;
	}
	return TT_SUCCESS;
}

// Delete the coredata reporting timer
void coredata_delete_timer(struct apex_info *app)
{
	if (app && app->coredata_timer)
		timer_delete(app->coredata_timer);
}
