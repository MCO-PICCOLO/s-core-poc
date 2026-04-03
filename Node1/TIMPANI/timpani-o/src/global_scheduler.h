/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include "task.h"
#include "sched_info.h"
#include "node_config.h"
#include <map>
#include <vector>
#include <string>
#include <memory>

// Type definitions for consistency with schedinfo_service.h
using NodeSchedInfoMap = std::map<std::string, sched_info_t>; // node_id -> sched_info_t

/**
 * @brief Global scheduler for task scheduling across multiple nodes
 *
 * Simplified version that works with gRPC schedinfo_server and NodeConfigManager.
 * Removed external dependencies and focuses on core scheduling algorithms.
 */
class GlobalScheduler {
public:
    /**
     * @brief Constructor
     * @param node_config_manager Node configuration manager for getting node/CPU info
     */
    explicit GlobalScheduler(std::shared_ptr<NodeConfigManager> node_config_manager);
    ~GlobalScheduler();

    /**
     * @brief Set tasks to be scheduled (called from gRPC server)
     * @param tasks Vector of tasks to schedule
     */
    void set_tasks(const std::vector<Task>& tasks);

    /**
     * @brief Execute scheduling algorithm
     * @param algorithm Scheduling algorithm ("best_fit_decreasing" or "least_loaded")
     * @return true if scheduling was successful, false otherwise
     */
    bool schedule(const std::string& algorithm = "best_fit_decreasing");

    /**
     * @brief Get the final schedules map (node_id -> sched_info_t)
     * @return Map of node schedules
     */
    const NodeSchedInfoMap& get_sched_info_map() const;

    /**
     * @brief Check if schedules are available
     * @return true if schedules exist, false otherwise
     */
    bool has_schedules() const;

    /**
     * @brief Get total number of scheduled tasks
     * @return Number of scheduled tasks
     */
    size_t get_total_scheduled_tasks() const;

    /**
     * @brief Clear all schedules and reset state
     */
    void clear();

private:
    // Node configuration manager
    std::shared_ptr<NodeConfigManager> node_config_manager_;

    // Available CPUs per node (updated from node configuration)
    std::map<std::string, std::vector<int>> available_cpus_per_node_;

    // CPU utilization tracking per node and CPU
    std::map<std::string, std::map<int, double>> cpu_utilization_per_node_;

    // Tasks to be scheduled
    std::vector<Task> tasks_;

    // Final schedule map (node_id -> sched_info_t)
    NodeSchedInfoMap sched_info_map_;

    // CPU_UTILIZATION_THRESHOLD
    static constexpr double CPU_UTILIZATION_THRESHOLD = 0.90;

    // New scheduling algorithm based on target node requirements
    void schedule_with_target_node_priority();

    // Core scheduling algorithms (legacy)
    void schedule_with_least_loaded();
    void schedule_with_best_fit_decreasing();

    // Schedule generation
    void generate_schedules();

    // New helper functions for target node scheduling
    int find_best_cpu_for_task(const Task& task, const std::string& node_id);
    std::vector<int> get_sorted_cpus_by_utilization(const std::string& node_id, bool prefer_high_utilization = true);
    bool assign_task_to_node_cpu(Task& task, const std::string& node_id, int cpu_id);

    // Algorithm helper functions (legacy)
    std::string find_best_node_least_loaded(const Task& task);
    std::string find_best_node_best_fit_decreasing(const Task& task);

    // Utility functions
    bool is_task_schedulable_on_node(const Task& task, const std::string& node_id);
    double calculate_node_utilization(const std::string& node_id, bool include_new_task = false, const Task* new_task = nullptr);
    double calculate_cpu_utilization(const std::string& node_id, int cpu_id);

    // Internal helper functions
    void initialize_available_cpus();
    void initialize_cpu_utilization_tracking();
    void cleanup_schedules();
    void print_scheduling_results();
    void print_node_details(const std::string& node_id);
};
