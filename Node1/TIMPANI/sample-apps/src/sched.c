/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#define _GNU_SOURCE
#include <sched.h>

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/syscall.h>

#include "libttsched.h"

#define PROCESS_NAME_SIZE	16

void set_affinity(int cpu) {
	cpu_set_t cpuset;
	CPU_ZERO(&cpuset);
	CPU_SET(cpu, &cpuset);

	// Set the current thread's (the main thread) CPU affinity mask
	if (sched_setaffinity(0, sizeof(cpu_set_t), &cpuset) == -1) {
		perror("Error: sched_setaffinity");
	}
}

static int sched_setattr(pid_t pid, const struct sched_attr *attr,
			unsigned int flags)
{
	return syscall(SYS_sched_setattr, pid, attr, flags);
}

void set_schedattr(pid_t pid, unsigned int priority, unsigned int policy) {
	struct sched_attr attr;

	memset(&attr, 0, sizeof(attr));
	attr.size = sizeof(struct sched_attr);
	attr.sched_priority = priority;
	attr.sched_policy = policy;

	if (sched_setattr(pid, &attr, 0) == -1) {
		perror("Error calling sched_setattr.");
	}
}

void get_process_name_by_pid(const int pid, char name[])
{
	if (name) {
		char procpath[60] = {};

		sprintf(procpath, "/proc/%d/comm",pid);

		FILE* f = fopen(procpath,"r");
		if (f) {
			size_t size;
			size = fread(name, sizeof(char), PROCESS_NAME_SIZE, f);
			if (size > 0) {
				if ('\n' == name[size-1])
					name[size-1] = '\0';
			}
			fclose(f);
		}
	}
}
