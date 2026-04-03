/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <time.h>
#include <unistd.h>
#include <stdio.h>
#include <sched.h>
#include <stdint.h>
#include <sys/prctl.h>
#include <sys/syscall.h>

#include "exprocs.h"

char pr_name[16];

clockid_t clockid = CLOCK_REALTIME;

unsigned int task_period;	// microseconds
unsigned int task_runtime;	// microseconds
unsigned long long jitter_cnt;

static uint64_t get_cpu_time(void)
{
	struct timespec ts;

	clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
	return ts_ns(ts);
}

// Function for real-time workload
static void do_workload(void)
{
	static struct timespec before;
	struct timespec now;
	uint64_t start_ns, runtime_ns;

	clock_gettime(clockid, &now);

	if (ts_ns(before)) {
		int jitter = diff(ts_ns(now), ts_ns(before)) / NSEC_PER_USEC - task_period;
		if (jitter < -100 || jitter > 100) {
			printf("%s: jitter(%llu) for execution: %d us\n", pr_name, ++jitter_cnt, jitter);
		}
	}

	if (task_runtime) {
		// Busy loop during the assigned runtime
		runtime_ns = task_runtime * NSEC_PER_USEC;
		start_ns = get_cpu_time();
		while(1) {
			if ((get_cpu_time() - start_ns) >= runtime_ns) break;
		}
	}

	before = now;
	//printf("[%lu]: %s end of handler\n", ns, pr_name);
}

int main(int argc, char *argv[])
{
	sigset_t sigset;
	int signal_received; // Variable to keep track of the received signals
	int pid = getpid();

	if (argc < 3) {
		fprintf(stderr, "Usage: %s name period_in_us [runtime_in_us]\n", argv[0]);
		exit(1);
	}

	task_period = atoi(argv[2]);
	if (argc > 3) {
		task_runtime = atoi(argv[3]);
	}

	prctl(PR_SET_NAME, (unsigned long)argv[1], 0, 0, 0);
	prctl(PR_GET_NAME, pr_name, 0, 0, 0);

	sigemptyset(&sigset);
	sigaddset(&sigset, SIGNO_TT);
	sigprocmask(SIG_BLOCK, &sigset, NULL);

	printf("%s (%d) with period %u ms & runtime %u ms is waiting for signal(%d)\n",
		pr_name, pid, task_period/1000, task_runtime/1000, SIGNO_TT);

	while (1) {
		if (sigwait(&sigset, &signal_received) == -1) {   // Wait for a signal from act.sa_mask
			perror("Failed to wait for signals");
			return EXIT_FAILURE;
		}

		if (signal_received != SIGNO_TT) {
			printf("signal %d is received!!!\n", signal_received);
			continue;  // If the received signal is not SIGNO_TT, continue waiting
		}

		do_workload();
	}

	return EXIT_SUCCESS;
}
