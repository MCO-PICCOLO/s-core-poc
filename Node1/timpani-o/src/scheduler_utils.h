#pragma once

#include <vector>
#include <cstdint>

namespace scheduler_utils {

/**
 * @brief Calculate the Greatest Common Divisor (GCD) of two numbers
 */
uint64_t calculateGCD(uint64_t a, uint64_t b);

/**
 * @brief Calculate the Least Common Multiple (LCM) of two numbers
 */
uint64_t calculateLCM(uint64_t a, uint64_t b);

/**
 * @brief Calculate the hyperperiod for a set of task periods
 * @param periods Vector of task periods in microseconds
 * @return Hyperperiod in microseconds
 */
uint64_t calculateHyperperiod(const std::vector<uint64_t>& periods);

} // namespace scheduler_utils
