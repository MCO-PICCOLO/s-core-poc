/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */


#include "hyperperiod_manager.h"
#include "tlog.h"
#include <algorithm>
#include <set>

HyperperiodManager::HyperperiodManager()
{
    TLOG_INFO("HyperperiodManager created");
}

HyperperiodManager::~HyperperiodManager()
{
    Clear();
}

uint64_t HyperperiodManager::CalculateHyperperiod(const std::string& workload_id,
                                                  const std::vector<Task>& tasks)
{
    if (tasks.empty()) {
        TLOG_WARN("No tasks provided for workload: ", workload_id);
        return 0;
    }

    // Filter tasks for this specific workload
    std::vector<Task> workload_tasks;
    for (const auto& task : tasks) {
        if (task.workload_id == workload_id) {
            workload_tasks.push_back(task);
        }
    }

    if (workload_tasks.empty()) {
        TLOG_WARN("No tasks found for workload: ", workload_id);
        return 0;
    }

    // Extract unique periods from workload tasks
    std::set<uint64_t> unique_periods;
    for (const auto& task : workload_tasks) {
        if (task.period_us > 0) {
            unique_periods.insert(task.period_us);
        }
    }

    if (unique_periods.empty()) {
        TLOG_ERROR("No valid periods found for workload: ", workload_id);
        return 0;
    }

    // Convert set to vector for LCM calculation
    std::vector<uint64_t> periods(unique_periods.begin(), unique_periods.end());

    // Calculate hyperperiod as LCM of all periods
    uint64_t hyperperiod = CalculateLcm(periods);

    // Store hyperperiod information
    HyperperiodInfo info;
    info.workload_id = workload_id;
    info.hyperperiod_us = hyperperiod;
    info.periods = periods;
    info.task_count = workload_tasks.size();

    hyperperiod_map_[workload_id] = info;

    TLOG_INFO("Calculated hyperperiod for workload '", workload_id, "':");
    TLOG_INFO("  Tasks: ", info.task_count);
    TLOG_INFO("  Unique periods: ", periods.size());
    for (uint64_t period : periods) {
        TLOG_INFO("    Period: ", period, " us (", period / 1000, " ms)");
    }
    TLOG_INFO("  Hyperperiod: ", hyperperiod, " us (", hyperperiod / 1000, " ms)");

    return hyperperiod;
}

const HyperperiodInfo* HyperperiodManager::GetHyperperiodInfo(const std::string& workload_id) const
{
    auto it = hyperperiod_map_.find(workload_id);
    return (it != hyperperiod_map_.end()) ? &it->second : nullptr;
}

const std::map<std::string, HyperperiodInfo>& HyperperiodManager::GetAllHyperperiods() const
{
    return hyperperiod_map_;
}

void HyperperiodManager::ClearWorkload(const std::string& workload_id)
{
    auto it = hyperperiod_map_.find(workload_id);
    if (it != hyperperiod_map_.end()) {
        TLOG_INFO("Cleared hyperperiod for workload: ", workload_id);
        hyperperiod_map_.erase(it);
    }
}

void HyperperiodManager::Clear()
{
    if (!hyperperiod_map_.empty()) {
        TLOG_INFO("Cleared all hyperperiod information for ", hyperperiod_map_.size(), " workloads");
        hyperperiod_map_.clear();
    }
}

bool HyperperiodManager::HasHyperperiod(const std::string& workload_id) const
{
    return hyperperiod_map_.find(workload_id) != hyperperiod_map_.end();
}

uint64_t HyperperiodManager::Gcd(uint64_t a, uint64_t b)
{
    while (b != 0) {
        uint64_t temp = b;
        b = a % b;
        a = temp;
    }
    return a;
}

uint64_t HyperperiodManager::Lcm(uint64_t a, uint64_t b)
{
    if (a == 0 || b == 0) {
        return 0;
    }
    return (a / Gcd(a, b)) * b;
}

uint64_t HyperperiodManager::CalculateLcm(const std::vector<uint64_t>& periods)
{
    if (periods.empty()) {
        return 0;
    }

    uint64_t result = periods[0];
    for (size_t i = 1; i < periods.size(); ++i) {
        result = Lcm(result, periods[i]);

        // Sanity check for extremely large hyperperiods
        if (result > 3600000000ULL) { // 1 hour in microseconds
            TLOG_WARN("Hyperperiod is very large: ", result / 1000000, " seconds");
            TLOG_WARN("This may indicate incompatible periods in the workload");
        }
    }

    return result;
}
