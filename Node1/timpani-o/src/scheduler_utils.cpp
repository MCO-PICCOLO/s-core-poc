#include "scheduler_utils.h"

#include <iostream>

namespace scheduler_utils {

uint64_t calculateGCD(uint64_t a, uint64_t b)
{
    while (b != 0) {
        uint64_t temp = b;
        b = a % b;
        a = temp;
    }
    return a;
}

uint64_t calculateLCM(uint64_t a, uint64_t b)
{
    return (a / calculateGCD(a, b)) * b;
}

uint64_t calculateHyperperiod(const std::vector<uint64_t>& periods)
{
    if (periods.empty()) return 0;
    uint64_t result = periods[0];
    for (size_t i = 1; i < periods.size(); ++i) {
        result = calculateLCM(result, periods[i]);
        // Prevent overflow - reasonable limit for hyperperiod
        if (result > 10000000) {  // 10 seconds max
            std::cerr << "Warning: Hyperperiod too large (" << result
                      << " us), may cause issues" << std::endl;
        }
    }
    return result;
}

}  // namespace scheduler_utils
