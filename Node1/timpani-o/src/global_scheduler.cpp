#include "global_scheduler.h"
#include "tlog.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <iomanip>
#include <iostream>

GlobalScheduler::GlobalScheduler(std::shared_ptr<NodeConfigManager> node_config_manager)
    : node_config_manager_(node_config_manager)
{
    TLOG_INFO("GlobalScheduler created with NodeConfigManager");
    if (node_config_manager_) {
        TLOG_DEBUG("NodeConfigManager pointer is valid");
        if (node_config_manager_->IsLoaded()) {
            TLOG_INFO("NodeConfigManager is loaded, initializing available CPUs");
            initialize_available_cpus();
            initialize_cpu_utilization_tracking();
        } else {
            TLOG_WARN("NodeConfigManager is not loaded yet");
        }
    } else {
        TLOG_ERROR("NodeConfigManager pointer is null!");
    }
}

GlobalScheduler::~GlobalScheduler()
{
    cleanup_schedules();
}

void GlobalScheduler::set_tasks(const std::vector<Task>& tasks)
{
    tasks_ = tasks;
    TLOG_INFO("GlobalScheduler: Set ", tasks_.size(), " tasks for scheduling");

    // Log task details
    for (const auto& task : tasks_) {
        TLOG_DEBUG("Task: ", task.name,
                  " | Workload: ", task.workload_id,
                  " | Target Node: ", task.target_node,
                  " | Priority: ", task.priority,
                  " | Period: ", task.period_us, "us",
                  " | Runtime: ", task.runtime_us, "us");
    }
}

bool GlobalScheduler::schedule(const std::string& algorithm)
{
    if (tasks_.empty()) {
        TLOG_ERROR("No tasks to schedule");
        return false;
    }

    if (!node_config_manager_ || !node_config_manager_->IsLoaded()) {
        TLOG_ERROR("Node configuration not available");
        if (!node_config_manager_) {
            TLOG_ERROR("  - NodeConfigManager pointer is null");
        } else {
            TLOG_ERROR("  - NodeConfigManager is not loaded");
        }
        return false;
    }

    // Clear previous schedules
    cleanup_schedules();

    // Reinitialize available CPUs and utilization tracking
    initialize_available_cpus();
    initialize_cpu_utilization_tracking();

    // Apply pre-loaded utilization from existing workloads (after zeroing)
    apply_preloaded_utilization();

    TLOG_INFO("=== Starting GlobalScheduler with algorithm: ", algorithm, " ===");
    TLOG_INFO("Tasks to schedule: ", tasks_.size());
    TLOG_INFO("Available nodes: ", available_cpus_per_node_.size());

    // Execute scheduling algorithm based on new requirements
    if (algorithm == "target_node_priority") {
        schedule_with_target_node_priority();
    } else if (algorithm == "least_loaded") {
        schedule_with_least_loaded();
    } else if (algorithm == "best_fit_decreasing") {
        schedule_with_best_fit_decreasing();
    } else {
        TLOG_ERROR("Unknown scheduling algorithm: ", algorithm);
        return false;
    }    // Generate final schedules
    generate_schedules();

    // Print results
    print_scheduling_results();

    return has_schedules();
}

void GlobalScheduler::schedule_with_least_loaded()
{
    TLOG_INFO("Executing Least Loaded scheduling algorithm");

    int scheduled_count = 0;

    for (auto& task : tasks_) {
        std::string best_node = find_best_node_least_loaded(task);

        if (!best_node.empty()) {
            task.assigned_node = best_node;
            task.assigned_cpu = available_cpus_per_node_[best_node].front();
            available_cpus_per_node_[best_node].erase(
                available_cpus_per_node_[best_node].begin());
            scheduled_count++;
            TLOG_INFO("  ✓ Task '", task.name, "' → Node '", best_node,
                      "' (CPU ", task.assigned_cpu, ")");
        } else {
            TLOG_WARN("  ✗ Task '", task.name,
                      "' could not be scheduled (no suitable node)");
        }
    }

    TLOG_INFO("Scheduled ", scheduled_count, "/", tasks_.size(), " tasks");
}

void GlobalScheduler::schedule_with_best_fit_decreasing()
{
    TLOG_INFO("Executing Best Fit Decreasing scheduling algorithm");

    // Sort tasks by execution time in decreasing order
    std::sort(tasks_.begin(), tasks_.end(), [](const Task& a, const Task& b) {
        return a.runtime_us > b.runtime_us;
    });

    int scheduled_count = 0;

    for (auto& task : tasks_) {
        std::string best_node;

        // Check if task has a target node specified
        if (!task.target_node.empty()) {
            // Try to schedule on the specified target node
            if (is_task_schedulable_on_node(task, task.target_node) &&
                !available_cpus_per_node_[task.target_node].empty()) {
                best_node = task.target_node;
                TLOG_DEBUG("Using target node ", task.target_node, " for task ", task.name);
            } else {
                TLOG_WARN("Target node ", task.target_node, " not available for task ", task.name);
            }
        } else {
            // Use normal best fit decreasing algorithm
            best_node = find_best_node_best_fit_decreasing(task);
        }

        if (!best_node.empty()) {
            task.assigned_node = best_node;

            // Handle CPU affinity
            if (task.affinity != "any" && !task.affinity.empty()) {
                try {
                    int required_cpu = std::stoi(task.affinity);
                    // Check if the required CPU is available
                    auto& available_cpus = available_cpus_per_node_[best_node];
                    auto cpu_it = std::find(available_cpus.begin(),
                                            available_cpus.end(), required_cpu);
                    if (cpu_it != available_cpus.end()) {
                        task.assigned_cpu = required_cpu;
                        available_cpus.erase(cpu_it);
                    } else {
                        // Required CPU not available, use any available CPU
                        task.assigned_cpu = available_cpus.front();
                        available_cpus.erase(available_cpus.begin());
                        TLOG_WARN("    ⚠ CPU ", required_cpu,
                                  " not available, using CPU ", task.assigned_cpu);
                    }
                } catch (const std::exception&) {
                    // Invalid affinity format, use any available CPU
                    task.assigned_cpu = available_cpus_per_node_[best_node].front();
                    available_cpus_per_node_[best_node].erase(
                        available_cpus_per_node_[best_node].begin());
                }
            } else {
                // Use any available CPU
                task.assigned_cpu = available_cpus_per_node_[best_node].front();
                available_cpus_per_node_[best_node].erase(
                    available_cpus_per_node_[best_node].begin());
            }

            scheduled_count++;
            TLOG_INFO("  ✓ Task '", task.name, "' → Node '", best_node,
                      "' (CPU ", task.assigned_cpu,
                      ", Exec=", task.runtime_us/1000, "ms)");
        } else {
            TLOG_WARN("  ✗ Task '", task.name, "' could not be scheduled");
            if (!task.target_node.empty()) {
                TLOG_WARN("    (target node '", task.target_node, "' not available)");
            } else {
                TLOG_WARN("    (no suitable node found)");
            }
        }
    }

    TLOG_INFO("Scheduled ", scheduled_count, "/", tasks_.size(), " tasks");
}

std::string GlobalScheduler::find_best_node_least_loaded(const Task& task)
{
    std::string best_node = "";
    double lowest_utilization = 1.0;

    for (const auto& pair : available_cpus_per_node_) {
        const std::string& node_id = pair.first;
        const std::vector<int>& cpus = pair.second;

        if (cpus.empty()) continue;
        if (!is_task_schedulable_on_node(task, node_id)) continue;

        double utilization = calculate_node_utilization(node_id);
        if (utilization < lowest_utilization) {
            lowest_utilization = utilization;
            best_node = node_id;
        }
    }

    return best_node;
}

std::string GlobalScheduler::find_best_node_best_fit_decreasing(const Task& task)
{
    std::string best_node = "";
    double best_fit_utilization = -1.0;

    for (const auto& pair : available_cpus_per_node_) {
        const std::string& node_id = pair.first;
        const std::vector<int>& cpus = pair.second;

        if (cpus.empty()) continue;
        if (!is_task_schedulable_on_node(task, node_id)) continue;

        double current_utilization = calculate_node_utilization(node_id);
        double new_utilization = calculate_node_utilization(node_id, true, &task);

        // Best fit: find the node that will have the highest utilization after
        // assignment but still be schedulable
        if (new_utilization <= 1.0 && new_utilization > best_fit_utilization) {
            best_fit_utilization = new_utilization;
            best_node = node_id;
        }
    }

    return best_node;
}

void GlobalScheduler::generate_schedules()
{
    TLOG_INFO("=== Generating Node Schedules ===");

    // Initialize schedules for all nodes that have tasks
    std::map<std::string, int> task_counts;
    for (const auto& task : tasks_) {
        if (!task.assigned_node.empty()) {
            task_counts[task.assigned_node]++;
        }
    }

    // Allocate memory and populate schedules
    for (const auto& pair : task_counts) {
        const std::string& node_id = pair.first;
        int count = pair.second;

        sched_info_map_[node_id].num_tasks = count;
        sched_info_map_[node_id].tasks = (sched_task_t*)malloc(sizeof(sched_task_t) * count);

        int task_index = 0;
        for (const auto& task : tasks_) {
            if (task.assigned_node == node_id) {
                sched_task_t& sched_task = sched_info_map_[node_id].tasks[task_index];

                strncpy(sched_task.task_name, task.name.c_str(),
                        sizeof(sched_task.task_name) - 1);
                sched_task.task_name[sizeof(sched_task.task_name) - 1] = '\0';

                sched_task.period_ns = task.period_us * 1000;      // Convert to nanoseconds
                sched_task.runtime_ns = task.runtime_us * 1000;    // Convert to nanoseconds
                sched_task.deadline_ns = task.deadline_us * 1000;  // Convert to nanoseconds
                sched_task.cpu_affinity = task.assigned_cpu;
                sched_task.sched_policy = task.policy;             // Default to FIFO
                sched_task.sched_priority = task.priority;
                sched_task.release_time = task.release_time; // Release time in microseconds
                sched_task.max_dmiss = task.max_dmiss;

                strncpy(sched_task.assigned_node, task.assigned_node.c_str(),
                        sizeof(sched_task.assigned_node) - 1);
                sched_task.assigned_node[sizeof(sched_task.assigned_node) - 1] = '\0';

                task_index++;
            }
        }

        TLOG_INFO("Generated schedule for node '", node_id, "' with ", count, " tasks");
    }
}

bool GlobalScheduler::is_task_schedulable_on_node(const Task& task, const std::string& node_id)
{
    // Check if node exists in configuration
    if (!node_config_manager_) {
        return true; // Allow if no config manager
    }

    const NodeConfig* node_config = node_config_manager_->GetNodeConfig(node_id);
    if (!node_config) {
        // Node not in configuration, but allow scheduling if CPUs are available
        return available_cpus_per_node_.find(node_id) != available_cpus_per_node_.end() &&
               !available_cpus_per_node_.at(node_id).empty();
    }

    // Check memory requirement
    if (task.memory_mb > node_config->max_memory_mb) {
        return false;
    }

    // Check CPU affinity
    if (task.affinity != "any" && !task.affinity.empty()) {
        try {
            int required_cpu = std::stoi(task.affinity);
            const auto& available_cpus = available_cpus_per_node_.at(node_id);
            return std::find(available_cpus.begin(), available_cpus.end(), required_cpu) != available_cpus.end();
        } catch (const std::exception&) {
            // Invalid affinity format, treat as "any"
        }
    }

    return true;
}

double GlobalScheduler::calculate_node_utilization(const std::string& node_id, bool include_new_task, const Task* new_task)
{
    double utilization = 0.0;

    // Calculate utilization from already assigned tasks
    for (const auto& task : tasks_) {
        if (task.assigned_node == node_id) {
            if (task.period_us > 0) {
                utilization += (double)task.runtime_us / task.period_us;
            }
        }
    }

    // Add new task if specified
    if (include_new_task && new_task && new_task->period_us > 0) {
        utilization += (double)new_task->runtime_us / new_task->period_us;
    }

    return utilization;
}

void GlobalScheduler::initialize_available_cpus()
{
    available_cpus_per_node_.clear();

    if (!node_config_manager_ || !node_config_manager_->IsLoaded()) {
        TLOG_WARN("No node configuration available, cannot initialize CPUs");
        return;
    }

    const auto& nodes = node_config_manager_->GetAllNodes();
    for (const auto& pair : nodes) {
        const std::string& node_id = pair.first;
        const NodeConfig& config = pair.second;

        available_cpus_per_node_[node_id] = config.available_cpus;
        TLOG_INFO("Initialized node '", node_id, "' with ", config.available_cpus.size(), " CPUs");
    }
}

void GlobalScheduler::print_scheduling_results()
{
    TLOG_INFO("=== GlobalScheduler Results ===");

    // Group tasks by workload for better reporting
    std::map<std::string, int> workload_task_counts;
    for (const auto& task : tasks_) {
        if (!task.assigned_node.empty()) {
            workload_task_counts[task.workload_id]++;
        }
    }

    // Print workload summary
    for (const auto& wl_entry : workload_task_counts) {
        TLOG_INFO("Workload '", wl_entry.first, "': ", wl_entry.second, " tasks scheduled");
    }

    for (const auto& pair : sched_info_map_) {
        const std::string& node_id = pair.first;
        const sched_info_t& schedule = pair.second;

        TLOG_INFO("Node: ", node_id, " (", schedule.num_tasks, " tasks)");

        if (schedule.num_tasks > 0) {
            for (int i = 0; i < schedule.num_tasks; i++) {
                const sched_task_t& task = schedule.tasks[i];
                // Find workload_id for this task
                std::string task_workload = "unknown";
                for (const auto& orig_task : tasks_) {
                    if (orig_task.name == task.task_name && orig_task.assigned_node == node_id) {
                        task_workload = orig_task.workload_id;
                        break;
                    }
                }
                TLOG_INFO("  Task: ", task.task_name, " (workload: ", task_workload, ")",
                         " | Period: ", task.period_ns / 1000000, "ms",
                         " | Runtime: ", task.runtime_ns / 1000000, "ms",
                         " | CPU: ", task.cpu_affinity,
                         " | Priority: ", task.sched_priority);
            }
            print_node_details(node_id);
        } else {
            TLOG_INFO("  (No tasks assigned)");
        }
    }
}

void GlobalScheduler::print_node_details(const std::string& node_id)
{
    double node_utilization = calculate_node_utilization(node_id);
    TLOG_INFO("  Node Utilization: ", (node_utilization * 100.0), "%");

    // Print CPU-level utilization details
    if (cpu_utilization_per_node_.find(node_id) != cpu_utilization_per_node_.end()) {
        for (const auto& cpu_pair : cpu_utilization_per_node_[node_id]) {
            int cpu_id = cpu_pair.first;
            double cpu_util = cpu_pair.second;
            if (cpu_util > 0.0) {
                TLOG_INFO("    CPU ", cpu_id, ": ", (cpu_util * 100.0), "% utilization");
            }
        }
    }

    auto it = available_cpus_per_node_.find(node_id);
    if (it != available_cpus_per_node_.end()) {
        size_t cpu_count = it->second.size();
        if (node_utilization > 1.0 * cpu_count) {
            TLOG_WARN("  ⚠ WARNING: Node is over-utilized!");
        } else if (node_utilization > CPU_UTILIZATION_THRESHOLD * cpu_count) {
            TLOG_WARN("  ⚠ Node is highly utilized");
        } else {
            TLOG_INFO("  ✓ Node utilization is acceptable");
        }
    } else {
        TLOG_WARN("  ⚠ Node ID '", node_id, "' not found in available_cpus_per_node_");
    }
}

void GlobalScheduler::cleanup_schedules()
{
    for (auto& pair : sched_info_map_) {
        if (pair.second.tasks) {
            free(pair.second.tasks);
            pair.second.tasks = nullptr;
        }
        pair.second.num_tasks = 0;
    }
    sched_info_map_.clear();
}

const NodeSchedInfoMap& GlobalScheduler::get_sched_info_map() const
{
    return sched_info_map_;
}

bool GlobalScheduler::has_schedules() const
{
    return !sched_info_map_.empty();
}

size_t GlobalScheduler::get_total_scheduled_tasks() const
{
    size_t total = 0;
    for (const auto& pair : sched_info_map_) {
        total += pair.second.num_tasks;
    }
    return total;
}

void GlobalScheduler::clear()
{
    cleanup_schedules();
    tasks_.clear();
    available_cpus_per_node_.clear();
    cpu_utilization_per_node_.clear();
    preloaded_schedules_.clear();
    TLOG_INFO("GlobalScheduler cleared");
}

void GlobalScheduler::preload_utilization(const std::vector<NodeSchedInfoMap>& existing_schedules)
{
    preloaded_schedules_ = existing_schedules;
    TLOG_DEBUG("Registered ", existing_schedules.size(),
              " existing workload(s) for utilization preload");
}

void GlobalScheduler::apply_preloaded_utilization()
{
    if (preloaded_schedules_.empty()) return;

    for (const auto& node_sched_info : preloaded_schedules_) {
        for (const auto& entry : node_sched_info) {
            const std::string& node_id = entry.first;
            const sched_info_t& sched_info = entry.second;

            for (int i = 0; i < sched_info.num_tasks; i++) {
                const sched_task_t& task = sched_info.tasks[i];
                if (task.period_ns > 0) {
                    double utilization =
                        static_cast<double>(task.runtime_ns) / task.period_ns;
                    int cpu_id = static_cast<int>(task.cpu_affinity);
                    if (cpu_utilization_per_node_.count(node_id) &&
                        cpu_utilization_per_node_[node_id].count(cpu_id)) {
                        cpu_utilization_per_node_[node_id][cpu_id] += utilization;
                    }
                }
            }
        }
    }

    TLOG_DEBUG("Applied pre-loaded utilization from ", preloaded_schedules_.size(),
              " existing workload(s)");
    preloaded_schedules_.clear();
}

void GlobalScheduler::schedule_with_target_node_priority()
{
    TLOG_INFO("Executing Target Node Priority scheduling algorithm");

    int scheduled_count = 0;

    for (auto& task : tasks_) {
        // Validate that task has both workload_id and target_node
        if (task.workload_id.empty()) {
            TLOG_ERROR("Task '", task.name, "' has no workload_id specified");
            continue;
        }

        // Rule 1: target_node must be assigned as assigned_node
        if (task.target_node.empty()) {
            TLOG_ERROR("Task '", task.name, "' (workload: ", task.workload_id, ") has no target_node specified");
            continue;
        }

        // Check if target node exists and has available CPUs
        if (available_cpus_per_node_.find(task.target_node) == available_cpus_per_node_.end()) {
            TLOG_ERROR("Target node '", task.target_node, "' not found in configuration");
            continue;
        }

        if (available_cpus_per_node_[task.target_node].empty()) {
            TLOG_WARN("Target node '", task.target_node, "' has no available CPUs for task '", task.name, "'");
            continue;
        }

        // Rule 1: Assign target_node as assigned_node
        task.assigned_node = task.target_node;

        // Rules 2 & 3: Find best CPU within target node
        int best_cpu = find_best_cpu_for_task(task, task.target_node);

        if (best_cpu != -1) {
            if (assign_task_to_node_cpu(task, task.target_node, best_cpu)) {
                scheduled_count++;
                TLOG_INFO("  ✓ Task '", task.name, "' → Node '", task.target_node,
                          "' (CPU ", best_cpu, ", Affinity: ", task.affinity, ")");
            } else {
                TLOG_WARN("  ✗ Failed to assign task '", task.name, "' to CPU ", best_cpu);
            }
        } else {
            TLOG_WARN("  ✗ No suitable CPU found for task '", task.name,
                      "' on target node '", task.target_node, "'");
        }
    }

    TLOG_INFO("Scheduled ", scheduled_count, "/", tasks_.size(), " tasks");
}

int GlobalScheduler::find_best_cpu_for_task(const Task& task, const std::string& node_id)
{
    auto& available_cpus = available_cpus_per_node_[node_id];
    if (available_cpus.empty()) {
        return -1;
    }

    double task_utilization = (task.period_us > 0) ?
                              (double)task.runtime_us / task.period_us : 0.0;

    // Rule 2: If task has specific CPU affinity, prioritize it
    if (task.affinity != "any" && !task.affinity.empty()) {
        try {
            int required_cpu = std::stoi(task.affinity);

            // Check if required CPU is available in target node
            auto cpu_it = std::find(available_cpus.begin(), available_cpus.end(), required_cpu);
            if (cpu_it != available_cpus.end()) {
                // Check utilization threshold
                double current_util = calculate_cpu_utilization(node_id, required_cpu);
                if (current_util + task_utilization <= CPU_UTILIZATION_THRESHOLD) {
                    TLOG_DEBUG("Using specific CPU affinity ", required_cpu, " for task ", task.name);
                    return required_cpu;
                } else {
                    TLOG_WARN("Required CPU ", required_cpu, " would exceed utilization threshold");
                }
            } else {
                TLOG_WARN("Required CPU ", required_cpu, " not available in node ", node_id);
            }
        } catch (const std::exception&) {
            TLOG_WARN("Invalid CPU affinity format: ", task.affinity, ", treating as 'any'");
        }
    }

    // Rule 3: For 'any' affinity, use Smart Packing Algorithm
    // Strategy: Start from highest CPU number, pack until 80% utilization
    std::vector<int> sorted_cpus = available_cpus;

    // Sort by CPU number (highest first) for consistent packing strategy
    std::sort(sorted_cpus.begin(), sorted_cpus.end(), std::greater<int>());

    // Find first CPU that can accommodate the task without exceeding threshold
    for (int cpu : sorted_cpus) {
        double current_util = calculate_cpu_utilization(node_id, cpu);
        if (current_util + task_utilization <= CPU_UTILIZATION_THRESHOLD) {
            TLOG_DEBUG("Selected CPU ", cpu, " for task ", task.name,
                      " (util: ", (current_util * 100), "% + ", (task_utilization * 100),
                      "% = ", ((current_util + task_utilization) * 100), "%)");
            return cpu;
        }
    }

    TLOG_WARN("No CPU can accommodate task ", task.name, " (requires ",
              (task_utilization * 100), "% utilization)");
    return -1;
}

std::vector<int> GlobalScheduler::get_sorted_cpus_by_utilization(const std::string& node_id, bool prefer_high_utilization)
{
    auto& available_cpus = available_cpus_per_node_[node_id];
    std::vector<int> sorted_cpus = available_cpus;

    // Sort CPUs by utilization and CPU number
    std::sort(sorted_cpus.begin(), sorted_cpus.end(),
        [this, &node_id, prefer_high_utilization](int cpu_a, int cpu_b) {
            double util_a = calculate_cpu_utilization(node_id, cpu_a);
            double util_b = calculate_cpu_utilization(node_id, cpu_b);

            // Primary criterion: utilization (prefer high utilization to pack tasks)
            if (std::abs(util_a - util_b) > 0.01) { // 1% threshold
                return prefer_high_utilization ? (util_a > util_b) : (util_a < util_b);
            }

            // Secondary criterion: prefer higher CPU numbers
            return cpu_a > cpu_b;
        });

    return sorted_cpus;
}

bool GlobalScheduler::assign_task_to_node_cpu(Task& task, const std::string& node_id, int cpu_id)
{
    auto& available_cpus = available_cpus_per_node_[node_id];

    // Check if CPU exists in available list
    auto cpu_it = std::find(available_cpus.begin(), available_cpus.end(), cpu_id);
    if (cpu_it == available_cpus.end()) {
        TLOG_ERROR("CPU ", cpu_id, " not found in available CPUs for node ", node_id);
        return false;
    }

    // Calculate new utilization if task is assigned
    double task_utilization = (task.period_us > 0) ?
                              (double)task.runtime_us / task.period_us : 0.0;
    double current_cpu_util = calculate_cpu_utilization(node_id, cpu_id);
    double new_cpu_util = current_cpu_util + task_utilization;

    // Check utilization threshold (90% max)
    if (new_cpu_util > CPU_UTILIZATION_THRESHOLD) {
        TLOG_WARN("CPU ", cpu_id, " utilization would exceed threshold (",
                  (new_cpu_util * 100), "% > ", (CPU_UTILIZATION_THRESHOLD * 100), "%)");
        return false;
    }

    // Assign to task
    task.assigned_cpu = cpu_id;

    // Update CPU utilization (don't remove from available - allow multiple tasks per CPU)
    cpu_utilization_per_node_[node_id][cpu_id] = new_cpu_util;

    TLOG_DEBUG("Assigned task '", task.name, "' to CPU ", cpu_id,
               " (utilization: ", (current_cpu_util * 100), "% → ", (new_cpu_util * 100), "%)");

    return true;
}

double GlobalScheduler::calculate_cpu_utilization(const std::string& node_id, int cpu_id)
{
    if (cpu_utilization_per_node_.find(node_id) == cpu_utilization_per_node_.end()) {
        return 0.0;
    }

    if (cpu_utilization_per_node_[node_id].find(cpu_id) == cpu_utilization_per_node_[node_id].end()) {
        return 0.0;
    }

    return cpu_utilization_per_node_[node_id][cpu_id];
}

void GlobalScheduler::initialize_cpu_utilization_tracking()
{
    cpu_utilization_per_node_.clear();

    for (const auto& pair : available_cpus_per_node_) {
        const std::string& node_id = pair.first;
        const std::vector<int>& cpus = pair.second;

        for (int cpu : cpus) {
            cpu_utilization_per_node_[node_id][cpu] = 0.0;
        }
    }

    TLOG_DEBUG("Initialized CPU utilization tracking for ",
               cpu_utilization_per_node_.size(), " nodes");
}
