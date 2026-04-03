/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef _SAMPLE_APPS_H
#define _SAMPLE_APPS_H

#include <time.h>
#include <stdbool.h>

#define SIGNO_TT		__SIGRTMIN+2
#define SIGNO_STOPTRACER	__SIGRTMIN+3

#define NSEC_PER_SEC		1000000000
#define USEC_PER_SEC		1000000
#define NSEC_PER_USEC		1000
#define MSEC_PER_SEC		1000
#define NSEC_PER_MSEC		1000000

#define ts_ns(a)	    	((a.tv_sec * NSEC_PER_SEC) + a.tv_nsec)
#define diff(b, a)	    	(b - a)

/* Realtime task configuration */
typedef struct {
    unsigned long period_ms;      /* Task period in milliseconds */
    unsigned long deadline_ms;    /* Task deadline in milliseconds */
    unsigned long runtime_us;     /* Expected runtime in microseconds */
    int priority;                 /* Real-time priority (1-99) */
    bool enable_stats;            /* Enable runtime statistics */
    char name[16];                /* Task name */
} rt_task_config_t;

/* Runtime statistics */
typedef struct {
    unsigned long min_runtime_us;
    unsigned long max_runtime_us;
    unsigned long avg_runtime_us;
    unsigned long total_runtime_us;
    unsigned long deadline_misses;
    unsigned long iterations;
    unsigned long last_runtime_us;
} rt_stats_t;

/* Function prototypes */
void rt_task_init(rt_task_config_t *config);
void rt_stats_init(rt_stats_t *stats);
void rt_stats_update(rt_stats_t *stats, unsigned long runtime_us, bool deadline_miss);
void rt_stats_print(const rt_stats_t *stats, const rt_task_config_t *config);

#endif /* _SAMPLE_APPS_H */
