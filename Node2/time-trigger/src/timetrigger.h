#ifndef _TIMETRIGGER_H
#define _TIMETRIGGER_H

#ifdef __cplusplus
extern "C" {
#endif

#define SIGNO_TT		__SIGRTMIN+2

// Time constant definitions (TT_ namespace)
#define TT_NSEC_PER_SEC         1000000000ULL
#define TT_USEC_PER_SEC         1000000ULL
#define TT_NSEC_PER_USEC        1000ULL

// Inline time conversion functions (improved API)
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

// Backward compatibility macros (for gradual migration)
#define NSEC_PER_SEC    TT_NSEC_PER_SEC
#define USEC_PER_SEC    TT_USEC_PER_SEC
#define NSEC_PER_USEC   TT_NSEC_PER_USEC

// Map legacy macros to new API (legacy support)
#define ts_ns(a)        tt_timespec_to_ns(&(a))
#define ts_us(a)        tt_timespec_to_us(&(a))
#define us_ts(us)       tt_us_to_timespec(us)
#define ns_ts(ns)       tt_ns_to_timespec(ns)
#define ts_diff(b, a)   tt_timespec_diff_ns(&(b), &(a))

#ifdef __cplusplus
}
#endif

#endif /* _TIMETRIGGER_H */
