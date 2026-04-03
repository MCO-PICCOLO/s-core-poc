/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef _LIBTTSCHED_H
#define _LIBTTSCHED_H

#ifdef __cplusplus
extern "C" {
#endif

struct sched_attr {
	uint32_t size;			/* Size of this structure */
	uint32_t sched_policy;		/* Policy (SCHED_*) */
	uint64_t sched_flags;		/* Flags */
	int32_t  sched_nice;		/* Nice value (SCHED_OTHER,
					   SCHED_BATCH) */
	uint32_t sched_priority;	/* Static priority (SCHED_FIFO,
					   SCHED_RR) */
	/* Remaining fields are for SCHED_DEADLINE */
	uint64_t sched_runtime;
	uint64_t sched_deadline;
	uint64_t sched_period;
};

void set_affinity(int cpu);
void set_schedattr(pid_t pid, unsigned int priority, unsigned int policy);
void get_process_name_by_pid(const int pid, char name[]);

#ifdef __cplusplus
}
#endif

#endif	/* _LIBTTSCHED_H */
