#ifndef _INTERNAL_H
#define _INTERNAL_H

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <signal.h>
#include <sched.h>
#include <time.h>
#include <getopt.h>
#include <sys/queue.h>
#include <errno.h>
#include <sys/time.h>
#include <sys/epoll.h>

#include "timetrigger.h"
#include "schedinfo.h"
#include <libtrpc.h>
#include "trace_bpf.h"

// ===== Integrated scheduling functions =====
// Functions imported from legacy libttsched

// TTSCHED error code system
typedef enum {
    TTSCHED_SUCCESS = 0,           // Success
    TTSCHED_ERROR_INVALID_ARGS = -1, // Invalid arguments
    TTSCHED_ERROR_PERMISSION = -2,   // Permission error
    TTSCHED_ERROR_SYSTEM = -3        // System error
} ttsched_error_t;

// Error message function
static inline const char* ttsched_error_string(ttsched_error_t error)
{
    switch (error) {
        case TTSCHED_SUCCESS: return "Success";
        case TTSCHED_ERROR_INVALID_ARGS: return "Invalid arguments";
        case TTSCHED_ERROR_PERMISSION: return "Permission denied";
        case TTSCHED_ERROR_SYSTEM: return "System error";
        default: return "Unknown error";
    }
}

struct sched_attr_tt {
    uint32_t size;
    uint32_t sched_policy;
    uint64_t sched_flags;
    int32_t  sched_nice;
    uint32_t sched_priority;
    uint64_t sched_runtime;
    uint64_t sched_deadline;
    uint64_t sched_period;
};

// Scheduling function declarations
ttsched_error_t set_affinity(pid_t pid, int cpu);
ttsched_error_t set_affinity_cpumask(pid_t pid, uint64_t cpumask);
ttsched_error_t set_affinity_cpumask_all_threads(pid_t pid, uint64_t cpumask);
ttsched_error_t set_schedattr(pid_t pid, unsigned int priority, unsigned int policy);
ttsched_error_t get_process_name_by_pid(const int pid, char name[]);
ttsched_error_t get_pid_by_name(const char *name, int *pid);
ttsched_error_t get_pid_by_nspid(const char *name, int nspid, int *pid);
ttsched_error_t create_pidfd(pid_t pid, int *pidfd);
ttsched_error_t send_signal_pidfd(int pidfd, int signal);
ttsched_error_t is_process_alive(int pidfd, int *alive);

// ===== BPF tracing functions =====

// ring_buffer callback function type from libbpf.h
typedef int (*ring_buffer_sample_fn)(void *ctx, void *data, size_t size);

// Only BPF tracing is used; ftrace-related functions have been removed

// BPF tracing function declarations

#ifdef CONFIG_TRACE_BPF
int bpf_on(ring_buffer_sample_fn sigwait_cb, ring_buffer_sample_fn schedstat_cb, void *ctx);
void bpf_off(void);
int bpf_add_pid(int pid);
int bpf_del_pid(int pid);
#else
static inline int bpf_on(ring_buffer_sample_fn sigwait_cb, ring_buffer_sample_fn schedstat_cb, void *ctx) { return 0; }
static inline void bpf_off(void) {}
static inline int bpf_add_pid(int pid) { return 0; }
static inline int bpf_del_pid(int pid) { return 0; }
#endif

// ===== TT system constant definitions =====
// All constants used in the Time Trigger system, managed under the TT_ namespace

// Timer-related constants
#define TT_TIMER_INCREMENT_NS        (5 * 1000 * 1000)   // 5ms - timer precision adjustment value

// Network communication constants
#define TT_POLLING_INTERVAL_US       (100 * 1000)        // 100ms - polling interval
#define TT_RETRY_INTERVAL_US         (1000 * 1000)       // 1s - retry interval
#define TT_MAX_CONNECTION_RETRIES    300                 // Maximum connection retry count

// Logging and statistics constants
#define TT_STATISTICS_LOG_INTERVAL   100                 // Statistics log output interval (based on hyperperiod cycles)

// ===== Log level system =====
typedef enum {
    TT_LOG_LEVEL_SILENT = 0,     // No log output
    TT_LOG_LEVEL_ERROR = 1,      // Errors only
    TT_LOG_LEVEL_WARNING = 2,    // Warnings and above
    TT_LOG_LEVEL_INFO = 3,       // Info and above (default)
    TT_LOG_LEVEL_DEBUG = 4,      // All logs
    TT_LOG_LEVEL_VERBOSE = 5     // Very detailed logs (may impact performance)
} tt_log_level_t;

// Global log level (default: INFO)
extern tt_log_level_t tt_global_log_level;

// Log level setter function
static inline void tt_set_log_level(tt_log_level_t level) {
    tt_global_log_level = level;
}

// Improved logging macros
#define TT_LOG_ERROR(fmt, ...) \
    do { \
        if (tt_global_log_level >= TT_LOG_LEVEL_ERROR) { \
            fprintf(stderr, "[ERROR] %s:%d: " fmt "\n", __func__, __LINE__, ##__VA_ARGS__); \
        } \
    } while(0)

#define TT_LOG_WARNING(fmt, ...) \
    do { \
        if (tt_global_log_level >= TT_LOG_LEVEL_WARNING) { \
            fprintf(stderr, "[WARNING] %s:%d: " fmt "\n", __func__, __LINE__, ##__VA_ARGS__); \
        } \
    } while(0)

#define TT_LOG_INFO(fmt, ...) \
    do { \
        if (tt_global_log_level >= TT_LOG_LEVEL_INFO) { \
            printf("[INFO] " fmt "\n", ##__VA_ARGS__); \
        } \
    } while(0)

#define TT_LOG_DEBUG(fmt, ...) \
    do { \
        if (tt_global_log_level >= TT_LOG_LEVEL_DEBUG) { \
            printf("[DEBUG] %s:%d: " fmt "\n", __func__, __LINE__, ##__VA_ARGS__); \
        } \
    } while(0)

// High-performance logging for timer handlers (optimized for frequent calls)
#define TT_LOG_TIMER(fmt, ...) \
    do { \
        if (unlikely(tt_global_log_level >= TT_LOG_LEVEL_VERBOSE)) { \
            printf("[TIMER] %s:%d: " fmt "\n", __func__, __LINE__, ##__VA_ARGS__); \
        } \
    } while(0)

#define TT_CHECK_ERROR(expr, error_code, fmt, ...) \
    do { \
        if (unlikely(!(expr))) { \
            TT_LOG_ERROR(fmt, ##__VA_ARGS__); \
            return error_code; \
        } \
    } while(0)

// Memory management macros
#define TT_MALLOC(ptr, type) \
    do { \
        (ptr) = malloc(sizeof(type)); \
        if (unlikely(!(ptr))) { \
            TT_LOG_ERROR("Failed to allocate memory for " #type); \
            return TT_ERROR_MEMORY; \
        } \
        memset((ptr), 0, sizeof(type)); \
    } while(0)

#define TT_CALLOC(ptr, count, type) \
    do { \
        (ptr) = calloc((count), sizeof(type)); \
        if (unlikely(!(ptr))) { \
            TT_LOG_ERROR("Failed to allocate memory for %zu " #type " items", (size_t)(count)); \
            return TT_ERROR_MEMORY; \
        } \
    } while(0)

#define TT_FREE(ptr) \
    do { \
        if (likely((ptr))) { \
            free((ptr)); \
            (ptr) = NULL; \
        } \
    } while(0)

#define TT_SAFE_FREE(ptr) \
    do { \
        free((ptr)); \
        (ptr) = NULL; \
    } while(0)

// Compiler hint macros
#ifndef likely
#define likely(x)   __builtin_expect(!!(x), 1)
#endif
#ifndef unlikely
#define unlikely(x) __builtin_expect(!!(x), 0)
#endif

// ===== TT error code system =====
// All functions return a unified tt_error_t type for consistent error handling
typedef enum {
    TT_SUCCESS = 0,              // Success
    TT_ERROR_MEMORY = -1,        // Memory allocation failure
    TT_ERROR_TIMER = -2,         // Timer-related error
    TT_ERROR_SIGNAL = -3,        // Signal handling error
    TT_ERROR_NETWORK = -4,       // Network communication error
    TT_ERROR_CONFIG = -5,        // Configuration error
    TT_ERROR_BPF = -6,           // BPF program error
    TT_ERROR_INVALID_ARGS = -7,  // Invalid arguments
    TT_ERROR_IO = -8,            // Input/output error
    TT_ERROR_PERMISSION = -9     // Permission error
} tt_error_t;

// Error message function
static inline const char* tt_error_string(tt_error_t error)
{
    switch (error) {
        case TT_SUCCESS: return "Success";
        case TT_ERROR_MEMORY: return "Memory allocation failed";
        case TT_ERROR_TIMER: return "Timer operation failed";
        case TT_ERROR_SIGNAL: return "Signal handling failed";
        case TT_ERROR_NETWORK: return "Network operation failed";
        case TT_ERROR_CONFIG: return "Configuration error";
        case TT_ERROR_BPF: return "BPF operation failed";
        case TT_ERROR_INVALID_ARGS: return "Invalid arguments";
        case TT_ERROR_IO: return "Input/Output error";
        case TT_ERROR_PERMISSION: return "Permission denied";
        default: return "Unknown error";
    }
}

// Time processing utility functions (using the unified API from timetrigger.h)
static inline void tt_timespec_add_us(struct timespec *ts, uint64_t us)
{
    uint64_t total_ns = tt_timespec_to_ns(ts) + (us * TT_NSEC_PER_USEC);
    *ts = tt_ns_to_timespec(total_ns);
}

// Forward declaration
struct context;

// Forward declaration
struct workload;

// Time trigger structure
struct time_trigger {
    timer_t timer;
    struct task_info task;
#ifdef CONFIG_TRACE_BPF
    uint64_t sigwait_ts;
    uint64_t sigwait_ts_prev;
    uint8_t sigwait_enter;
#endif
    struct timespec prev_timer;
    struct context *ctx;      // context pointer
    struct workload *workload; // owning workload pointer
    LIST_ENTRY(time_trigger) entry;
};

// Hyperperiod management structure (memory alignment optimized)
struct hyperperiod_manager {
    // Frequently accessed fields placed first
    uint64_t hyperperiod_us;
    uint64_t current_cycle;
    uint64_t hyperperiod_start_time_us;
    uint64_t completed_cycles;

    // Pointers
    struct time_trigger *tt_list;
    struct context *ctx;

    // Timer-related
    timer_t hyperperiod_timer;
    struct timespec hyperperiod_start_ts;

    // Statistics (32-bit)
    uint32_t tasks_in_hyperperiod;
    uint32_t total_deadline_misses;
    uint32_t cycle_deadline_misses;
    uint32_t _padding;  // padding for 8-byte alignment

    // Strings (placed last)
    char workload_id[64];
} __attribute__((packed, aligned(8)));

LIST_HEAD(listhead, time_trigger);

// ===== Workload structure =====
// Per-workload management structure for multi-workload support
// Each workload owns its own sched_info, hyperperiod_manager, and task list
struct workload {
    struct sched_info sched_info;           // Per-workload scheduling info (task linked list)
    struct hyperperiod_manager hp_manager;  // Per-workload hyperperiod timer and statistics
    struct listhead tt_list;               // Per-workload time_trigger list
    int nr_active_tasks;                   // Number of successfully initialized tasks
    struct context *ctx;                   // Context back-reference pointer
    LIST_ENTRY(workload) entry;            // Context workload linked list entry
};

LIST_HEAD(workload_listhead, workload);

// Structure for Apex.OS Task Info
#define MAX_APEX_NAME_LEN 256

struct apex_info {
    struct task_info task;
    char name[MAX_APEX_NAME_LEN];
    int nspid;
    uint64_t dmiss_time_us;
    int dmiss_count;
    timer_t coredata_timer;
    LIST_ENTRY(apex_info) entry;
};

LIST_HEAD(apex_listhead, apex_info);

// ===== TT system context structure =====
// Centralized context management replacing global variables
// Consolidates all state and configuration needed by every module into a single structure
struct context {
    // System configuration (initialized in config.c)
    struct {
        int cpu;                        // CPU binding number
        int prio;                       // Scheduling priority
        int port;                       // Network port
        const char *addr;               // Server address
        char node_id[TINFO_NODEID_MAX]; // Node identifier
        bool enable_sync;               // Timer synchronization enabled
        bool enable_plot;               // Plot feature enabled
        bool enable_apex;               // Apex.OS Test Mode
        clockid_t clockid;              // Clock type to use
        tt_log_level_t log_level;       // Log level
    } config;

    // Runtime state (dynamic state that changes during execution)
    struct {
        struct workload_listhead workloads; // Workload list (multi-workload support)
        uint32_t nr_workloads;              // Number of workloads
        volatile sig_atomic_t shutdown_requested; // Shutdown request flag
        struct timespec starttimer_ts;  // Start timer timestamp
        struct apex_listhead apex_list; // Apex.OS Task List
    } runtime;

    // Communication (D-Bus, event loop)
    struct {
        sd_event *event;                // systemd event loop
        sd_bus *dbus;                   // D-Bus connection
        int apex_fd;                    // Apex.OS Monitor Socket FD
    } comm;
};

// ===== TT system function declarations =====
// Systematically organized function interfaces by module

// ===== Configuration management (config.c) =====
tt_error_t parse_config(int argc, char *argv[], struct context *ctx);
tt_error_t validate_config(const struct context *ctx);

// ===== Core engine (core.c) =====
void timer_expired_handler(union sigval value);
tt_error_t start_timers(struct context *ctx);
tt_error_t epoll_loop(struct context *ctx);
tt_error_t handle_sigwait_bpf_event(void *ctx, void *data, size_t len);
tt_error_t handle_schedstat_bpf_event(void *ctx, void *data, size_t len);

// ===== Hyperperiod management (hyperperiod.c) =====
tt_error_t init_hyperperiod(struct context *ctx, const char *workload_id, uint64_t hyperperiod_us, struct hyperperiod_manager *hp_mgr);
void hyperperiod_cycle_handler(union sigval value);
uint64_t get_hyperperiod_relative_time(const struct hyperperiod_manager *hp_mgr);
void log_hyperperiod_statistics(const struct hyperperiod_manager *hp_mgr);
tt_error_t start_hyperperiod_timer(struct workload *wl);

// ===== Task management (task.c) =====
tt_error_t init_task_list(struct workload *wl);
void destroy_task_info_list(struct task_info *tasks);

// ===== Network communication (trpc.c) =====
tt_error_t init_trpc(struct context *ctx);
tt_error_t sync_timer_with_server(struct context *ctx);
tt_error_t deserialize_sched_info(struct context *ctx, serial_buf_t *sbuf, struct sched_info *sinfo, struct hyperperiod_manager *hp_mgr);
tt_error_t deserialize_workloads(struct context *ctx, serial_buf_t *sbuf);
tt_error_t report_deadline_miss(struct context *ctx, const char *taskname);

// ===== Signal handling (signal.c) =====
tt_error_t setup_signal_handlers(struct context *ctx);

// ===== Resource cleanup (cleanup.c) =====
void cleanup_context(struct context *ctx);

// ===== Utility functions =====
tt_error_t calibrate_bpf_time_offset(void);

// ====== Apex.OS Monitor (apex_monitor.c) =====
enum {
  APEX_FAULT = 0,
  APEX_UP = 1,
  APEX_DOWN = 2,
  APEX_RESET = 3,
};

int apex_monitor_init(struct context *ctx);
void apex_monitor_cleanup(struct context *ctx);
int apex_monitor_recv(struct context *ctx, char *name, int size, int *pid, int *type);
tt_error_t init_apex_list(struct context *ctx);
tt_error_t coredata_client_send(struct apex_info *app);
tt_error_t coredata_create_timer(struct apex_info *app);
void coredata_delete_timer(struct apex_info *app);

#endif /* _INTERNAL_H */
