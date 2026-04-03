#ifndef _TIMETRIGGER_H
#define _TIMETRIGGER_H

#ifdef __cplusplus
extern "C" {
#endif

/* Use glibc user-visible SIGRTMIN (=34), not __SIGRTMIN (=32).
 * glibc reserves __SIGRTMIN+0 (32) and __SIGRTMIN+1 (33) for NPTL internal use.
 * All user apps (including Rust libc::SIGRTMIN()) use SIGRTMIN = 34.
 * So SIGNO_TT = SIGRTMIN+2 = 36, matching sea_app's libc::SIGRTMIN()+2. */
#define SIGNO_TT		(SIGRTMIN+2)

// 시간 상수 정의 (TT_ namespace)
#define TT_NSEC_PER_SEC         1000000000ULL
#define TT_USEC_PER_SEC         1000000ULL
#define TT_NSEC_PER_USEC        1000ULL

// 인라인 시간 변환 함수들 (개선된 API)
static inline uint64_t tt_timespec_to_ns(const struct timespec *ts)
{
    return ((uint64_t)ts->tv_sec * TT_NSEC_PER_SEC) + ts->tv_nsec;
}

static inline uint64_t tt_timespec_to_us(const struct timespec *ts)
{
    return ((uint64_t)ts->tv_sec * TT_USEC_PER_SEC) + (ts->tv_nsec / TT_NSEC_PER_USEC);
}

static inline struct timespec tt_us_to_timespec(uint64_t us)
{
    struct timespec ts;
    ts.tv_sec = us / TT_USEC_PER_SEC;
    ts.tv_nsec = (us % TT_USEC_PER_SEC) * TT_NSEC_PER_USEC;
    return ts;
}

static inline struct timespec tt_ns_to_timespec(uint64_t ns)
{
    struct timespec ts;
    ts.tv_sec = ns / TT_NSEC_PER_SEC;
    ts.tv_nsec = ns % TT_NSEC_PER_SEC;
    return ts;
}

static inline int tt_timespec_compare(const struct timespec *a, const struct timespec *b)
{
    if (a->tv_sec != b->tv_sec) {
        return (a->tv_sec > b->tv_sec) ? 1 : -1;
    }
    return (a->tv_nsec > b->tv_nsec) ? 1 : ((a->tv_nsec < b->tv_nsec) ? -1 : 0);
}

static inline uint64_t tt_timespec_diff_ns(const struct timespec *b, const struct timespec *a)
{
    return tt_timespec_to_ns(b) - tt_timespec_to_ns(a);
}

// 하위 호환성을 위한 매크로 (점진적 마이그레이션용)
#define NSEC_PER_SEC    TT_NSEC_PER_SEC
#define USEC_PER_SEC    TT_USEC_PER_SEC
#define NSEC_PER_USEC   TT_NSEC_PER_USEC

// 기존 매크로를 새로운 API로 매핑 (레거시 지원)
#define ts_ns(a)        tt_timespec_to_ns(&(a))
#define ts_us(a)        tt_timespec_to_us(&(a))
#define us_ts(us)       tt_us_to_timespec(us)
#define ns_ts(ns)       tt_ns_to_timespec(ns)
#define ts_diff(b, a)   tt_timespec_diff_ns(&(b), &(a))

#ifdef __cplusplus
}
#endif

#endif /* _TIMETRIGGER_H */
