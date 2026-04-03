#ifndef HYPERPERIOD_MANAGER_H
#define HYPERPERIOD_MANAGER_H

#include <string>
#include <vector>
#include <map>
#include <cstdint>
#include "task.h"

/**
 * @brief Hyperperiod information for a workload
 */
struct HyperperiodInfo {
    std::string workload_id;
    uint64_t hyperperiod_us;        // Hyperperiod in microseconds
    std::vector<uint64_t> periods;  // All unique periods in this workload
    size_t task_count;              // Number of tasks in this workload
    
    HyperperiodInfo() : hyperperiod_us(0), task_count(0) {}
};

/**
 * @brief Manager for calculating and tracking hyperperiods per workload
 */
class HyperperiodManager {
public:
    HyperperiodManager();
    ~HyperperiodManager();

    /**
     * @brief Calculate hyperperiod for a workload based on task periods
     * @param workload_id The workload identifier
     * @param tasks Vector of tasks belonging to this workload
     * @return Calculated hyperperiod in microseconds
     */
    uint64_t CalculateHyperperiod(const std::string& workload_id, 
                                  const std::vector<Task>& tasks);

    /**
     * @brief Get hyperperiod information for a specific workload
     * @param workload_id The workload identifier
     * @return Pointer to HyperperiodInfo or nullptr if not found
     */
    const HyperperiodInfo* GetHyperperiodInfo(const std::string& workload_id) const;

    /**
     * @brief Get all hyperperiod information
     * @return Map of workload_id to HyperperiodInfo
     */
    const std::map<std::string, HyperperiodInfo>& GetAllHyperperiods() const;

    /**
     * @brief Clear hyperperiod information for a specific workload
     * @param workload_id The workload identifier
     */
    void ClearWorkload(const std::string& workload_id);

    /**
     * @brief Clear all hyperperiod information
     */
    void Clear();

    /**
     * @brief Check if hyperperiod exists for a workload
     * @param workload_id The workload identifier
     * @return True if hyperperiod exists, false otherwise
     */
    bool HasHyperperiod(const std::string& workload_id) const;

private:
    /**
     * @brief Calculate greatest common divisor (GCD) of two numbers
     * @param a First number
     * @param b Second number
     * @return GCD of a and b
     */
    uint64_t Gcd(uint64_t a, uint64_t b);

    /**
     * @brief Calculate least common multiple (LCM) of two numbers
     * @param a First number
     * @param b Second number
     * @return LCM of a and b
     */
    uint64_t Lcm(uint64_t a, uint64_t b);

    /**
     * @brief Calculate LCM of a vector of numbers
     * @param periods Vector of periods
     * @return LCM of all periods
     */
    uint64_t CalculateLcm(const std::vector<uint64_t>& periods);

    // Storage for workload hyperperiod information
    std::map<std::string, HyperperiodInfo> hyperperiod_map_;
};

#endif // HYPERPERIOD_MANAGER_H
