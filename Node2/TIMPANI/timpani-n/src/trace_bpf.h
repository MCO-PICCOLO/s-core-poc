/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef _TRACE_APP_H_
#define _TRACE_APP_H_

#ifdef __cplusplus
extern "C" {
#endif

struct sigwait_event {
	int pid;
	int tgid;
	uint64_t timestamp;
	uint8_t enter;
};

struct schedstat_event {
	int pid;
	int cpu;
	uint64_t ts_wakeup;
	uint64_t ts_start;
	uint64_t ts_stop;
};

#ifdef __cplusplus
}
#endif

#endif /* _TRACE_APP_H_ */
