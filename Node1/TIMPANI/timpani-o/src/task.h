/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef TASK_H
#define TASK_H

#include <string>
#include <stdint.h>

/**
 * @brief Internal task representation for scheduling
 */
struct Task {
    // Basic task information
    std::string name;
    std::string workload_id;         // Workload ID this task belongs to
    std::string target_node;         // Target node ID (can be empty for auto-assignment)
    int priority;
    int policy;              // Scheduling policy as integer (0: SCHED_NORMAL, 1: SCHED_FIFO, 2: SCHED_RR)

    // Timing parameters
    uint64_t period_us;              // Period in microseconds
    uint64_t runtime_us;             // Runtime in microseconds
    uint64_t deadline_us;            // Deadline in microseconds

    // Legacy timing (milliseconds) - for compatibility
    int execution_time;              // Execution time in milliseconds
    int deadline;                    // Deadline in milliseconds
    int period;                      // Period in milliseconds
    int release_time;                // release time in milliseconds
    int max_dmiss;                   // allowable max deadline misses

    // Resource requirements
    std::string affinity;            // CPU affinity ("any" or CPU ID as string)
    int cpu_affinity;                // CPU affinity as integer (-1 for any)
    int memory_mb;                   // Memory requirement in MB
    std::string dependencies;        // Task dependencies
    std::string cluster_requirement; // Cluster requirement

    // Assignment results (filled during scheduling)
    std::string assigned_node;       // Assigned node ID
    int assigned_cpu;                // Assigned CPU ID

    Task() : priority(0), period_us(0), runtime_us(0), deadline_us(0),
             execution_time(0), deadline(0), period(0),
             cpu_affinity(-1), memory_mb(64), assigned_cpu(-1) {}
};

#endif // TASK_H
