/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

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
#include <pthread.h>

#include "timetrigger.h"
#include "schedinfo.h"
#include <libtrpc.h>
#include "trace_bpf.h"

// ===== 통합된 스케줄링 함수들 =====
// 기존 libttsched에서 가져온 함수들

// TTSCHED 에러 코드 시스템
typedef enum {
    TTSCHED_SUCCESS = 0,              // 성공
    TTSCHED_ERROR_INVALID_ARGS = -1,  // 잘못된 인자
    TTSCHED_ERROR_PERMISSION = -2,    // 권한 오류
    TTSCHED_ERROR_SYSTEM = -3,        // 시스템 오류
    TTSCHED_ERROR_NO_PROCESS = -4     // 프로세스가 존재하지 않음 (ESRCH)
} ttsched_error_t;

// 에러 메시지 함수
static inline const char* ttsched_error_string(ttsched_error_t error)
{
    switch (error) {
        case TTSCHED_SUCCESS: return "Success";
        case TTSCHED_ERROR_INVALID_ARGS: return "Invalid arguments";
        case TTSCHED_ERROR_PERMISSION: return "Permission denied";
        case TTSCHED_ERROR_SYSTEM: return "System error";
        case TTSCHED_ERROR_NO_PROCESS: return "No such process";
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

// 스케줄링 함수 선언
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

// ===== BPF 트레이싱 함수들 =====

// ring_buffer callback function type from libbpf.h
typedef int (*ring_buffer_sample_fn)(void *ctx, void *data, size_t size);

// BPF 트레이싱만 사용하므로 ftrace 관련 함수들 제거됨

// BPF 트레이싱 함수 선언

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

// ===== TT 시스템 상수 정의 =====
// Time Trigger 시스템에서 사용하는 모든 상수들을 TT_ 네임스페이스로 관리

// 타이머 관련 상수
#define TT_TIMER_INCREMENT_NS        (5 * 1000 * 1000)   // 5ms - 타이머 정밀도 조정 값

// 네트워크 통신 상수
#define TT_POLLING_INTERVAL_US       (100 * 1000)        // 100ms - 폴링 간격
#define TT_RETRY_INTERVAL_US         (1000 * 1000)       // 1s - 재시도 간격
#define TT_MAX_CONNECTION_RETRIES    300                 // 최대 연결 재시도 횟수

// 로깅 및 통계 상수
#define TT_STATISTICS_LOG_INTERVAL   100                 // 통계 로그 출력 주기 (하이퍼피리어드 사이클 기준)

// ===== 로그 레벨 시스템 =====
typedef enum {
    TT_LOG_LEVEL_SILENT = 0,     // 로그 출력 없음
    TT_LOG_LEVEL_ERROR = 1,      // 에러만
    TT_LOG_LEVEL_WARNING = 2,    // 경고 이상
    TT_LOG_LEVEL_INFO = 3,       // 정보 이상 (기본값)
    TT_LOG_LEVEL_DEBUG = 4,      // 모든 로그
    TT_LOG_LEVEL_VERBOSE = 5     // 매우 상세한 로그 (성능 영향)
} tt_log_level_t;

// 전역 로그 레벨 (기본값: INFO)
extern tt_log_level_t tt_global_log_level;

// 로그 레벨 설정 함수
static inline void tt_set_log_level(tt_log_level_t level) {
    tt_global_log_level = level;
}

// 개선된 로깅 매크로
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

// 타이머 핸들러용 고성능 로깅 (빈번한 호출 시 성능 최적화)
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

// 메모리 관리 매크로
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

// 컴파일러 힌트 매크로
#ifndef likely
#define likely(x)   __builtin_expect(!!(x), 1)
#endif
#ifndef unlikely
#define unlikely(x) __builtin_expect(!!(x), 0)
#endif

// ===== TT 에러 코드 시스템 =====
// 모든 함수는 통일된 tt_error_t 타입을 반환하여 일관된 에러 처리 제공
typedef enum {
    TT_SUCCESS = 0,              // 성공
    TT_ERROR_MEMORY = -1,        // 메모리 할당 실패
    TT_ERROR_TIMER = -2,         // 타이머 관련 오류
    TT_ERROR_SIGNAL = -3,        // 시그널 처리 오류
    TT_ERROR_NETWORK = -4,       // 네트워크 통신 오류
    TT_ERROR_CONFIG = -5,        // 설정 관련 오류
    TT_ERROR_BPF = -6,           // BPF 프로그램 오류
    TT_ERROR_INVALID_ARGS = -7,  // 잘못된 인자
    TT_ERROR_IO = -8,            // 입출력 오류
    TT_ERROR_PERMISSION = -9     // 권한 오류
} tt_error_t;

// 에러 메시지 함수
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

// 시간 처리 유틸리티 함수 (timetrigger.h의 통합 API 사용)
static inline void tt_timespec_add_us(struct timespec *ts, uint64_t us)
{
    uint64_t total_ns = tt_timespec_to_ns(ts) + (us * TT_NSEC_PER_USEC);
    *ts = tt_ns_to_timespec(total_ns);
}

// Forward declaration
struct context;

// Time trigger 구조체
struct time_trigger {
    timer_t timer;
    struct task_info task;
#ifdef CONFIG_TRACE_BPF
    uint64_t sigwait_ts;
    uint64_t sigwait_ts_prev;
    uint8_t sigwait_enter;
#endif
    struct timespec prev_timer;
    struct context *ctx;  // context 포인터 추가
    LIST_ENTRY(time_trigger) entry;
};

// Hyperperiod 관리 구조체 (메모리 정렬 최적화)
struct hyperperiod_manager {
    // 자주 접근하는 필드들을 앞으로
    uint64_t hyperperiod_us;
    uint64_t current_cycle;
    uint64_t hyperperiod_start_time_us;
    uint64_t completed_cycles;

    // 포인터들
    struct time_trigger *tt_list;
    struct context *ctx;

    // 타이머 관련
    timer_t hyperperiod_timer;
    struct timespec hyperperiod_start_ts;

    // 통계 (32비트)
    uint32_t tasks_in_hyperperiod;
    uint32_t total_deadline_misses;
    uint32_t cycle_deadline_misses;
    uint32_t _padding;  // 8바이트 정렬을 위한 패딩

    // 문자열 (마지막에 배치)
    char workload_id[64];
} __attribute__((packed, aligned(8)));

LIST_HEAD(listhead, time_trigger);

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

// ===== TT 시스템 컨텍스트 구조체 =====
// 전역 변수를 대체하는 중앙화된 컨텍스트 관리
// 모든 모듈에서 필요한 상태와 설정을 하나의 구조체로 통합
struct context {
    // 시스템 설정 (config.c에서 초기화)
    struct {
        int cpu;                        // CPU 바인딩 번호
        int prio;                       // 스케줄링 우선순위
        int port;                       // 네트워크 포트
        const char *addr;               // 서버 주소
        char node_id[TINFO_NODEID_MAX]; // 노드 식별자
        bool enable_sync;               // 타이머 동기화 활성화
        bool enable_plot;               // 플롯 기능 활성화
	bool enable_apex;               // Apex.OS Test Mode
        clockid_t clockid;              // 사용할 클록 타입
        tt_log_level_t log_level;       // 로그 레벨
    } config;

    // 런타임 상태 (실행 중 변경되는 동적 상태)
    struct {
        struct listhead tt_list;        // 시간 트리거 태스크 목록
        struct sched_info sched_info;   // 스케줄링 정보
        volatile sig_atomic_t shutdown_requested; // 종료 요청 플래그
        struct timespec starttimer_ts;  // 시작 타이머 타임스탬프
        struct apex_listhead apex_list; // Apex.OS Task List
    } runtime;

    // 통신 관련 (D-Bus, 이벤트 루프)
    struct {
        sd_event *event;                // systemd 이벤트 루프
        sd_bus *dbus;                   // D-Bus 연결
        int apex_fd;                    // Apex.OS Monitor Socket FD
        pthread_mutex_t dbus_mutex;     // Serialises concurrent sd_bus_call_method
                                        // invocations from timer threads vs epoll_loop
    } comm;

    // 하이퍼피리어드 관리자 (hyperperiod.c에서 관리)
    struct hyperperiod_manager hp_manager;
};

// ===== TT 시스템 함수 선언 =====
// 모듈별로 체계적으로 정리된 함수 인터페이스

// ===== 설정 관리 (config.c) =====
tt_error_t parse_config(int argc, char *argv[], struct context *ctx);
tt_error_t validate_config(const struct context *ctx);

// ===== 코어 엔진 (core.c) =====
void timer_expired_handler(union sigval value);
tt_error_t start_timers(struct context *ctx);
tt_error_t epoll_loop(struct context *ctx);
tt_error_t handle_sigwait_bpf_event(void *ctx, void *data, size_t len);
tt_error_t handle_schedstat_bpf_event(void *ctx, void *data, size_t len);

// ===== 하이퍼피리어드 관리 (hyperperiod.c) =====
tt_error_t init_hyperperiod(struct context *ctx, const char *workload_id, uint64_t hyperperiod_us, struct hyperperiod_manager *hp_mgr);
void hyperperiod_cycle_handler(union sigval value);
uint64_t get_hyperperiod_relative_time(const struct hyperperiod_manager *hp_mgr);
void log_hyperperiod_statistics(const struct hyperperiod_manager *hp_mgr);
tt_error_t start_hyperperiod_timer(struct context *ctx);

// ===== 태스크 관리 (task.c) =====
tt_error_t init_task_list(struct context *ctx);
void destroy_task_info_list(struct task_info *tasks);
void reapply_affinities(struct context *ctx, struct sched_info *new_sinfo);

// ===== 네트워크 통신 (trpc.c) =====
tt_error_t init_trpc(struct context *ctx);
tt_error_t sync_timer_with_server(struct context *ctx);
tt_error_t deserialize_sched_info(struct context *ctx, serial_buf_t *sbuf, struct sched_info *sinfo);
tt_error_t poll_sched_info(struct context *ctx, struct sched_info *sinfo);
tt_error_t report_deadline_miss(struct context *ctx, const char *taskname);

// ===== 시그널 처리 (signal.c) =====
tt_error_t setup_signal_handlers(struct context *ctx);

// ===== 리소스 정리 (cleanup.c) =====
void cleanup_context(struct context *ctx);

// ===== 유틸리티 함수들 =====
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
