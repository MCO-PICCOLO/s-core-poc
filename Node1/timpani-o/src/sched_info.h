#ifndef SCHED_INFO_H
#define SCHED_INFO_H

#include <stdint.h>
#include <sched.h>

/**
 * @brief Individual task schedule information
 */
typedef struct {
    char task_name[16];          // Task name
    uint64_t period_ns;          // Period in nanoseconds
    uint64_t runtime_ns;         // Runtime in nanoseconds
    uint64_t deadline_ns;        // Deadline in nanoseconds
    int release_time;            // Release Time in us
    uint64_t cpu_affinity;       // CPU affinity (CPU ID)
    int sched_policy;            // Scheduling policy (SCHED_FIFO, SCHED_RR, etc.)
    int sched_priority;          // Scheduling priority
    int max_dmiss;               // Allowable max deadline misses
    char assigned_node[64];      // Assigned node
} sched_task_t;

/**
 * @brief Node schedule information containing multiple tasks
 */
typedef struct {
    int num_tasks;               // Number of tasks in this schedule
    sched_task_t* tasks;         // Array of tasks for this node
} sched_info_t;

#endif // SCHED_INFO_H
