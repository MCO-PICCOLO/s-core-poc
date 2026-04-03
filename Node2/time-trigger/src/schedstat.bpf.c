#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Include common header file between bpf and user-space programs
#include "trace_bpf.h"

// Ring buffer for notification events to user-space program
struct {
	__uint(type, BPF_MAP_TYPE_RINGBUF);
	__uint(max_entries, 64 * 4096);
} buffer SEC(".maps");

// Hash map for target pids to collect events from
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 1024);
	__type(key, int);
	__type(value, uint8_t);
} pid_filter_map SEC(".maps");

// BPF program's internal data structure to store specific scheduler statistics
struct sched_time {
	uint64_t ts_wakeup;
	uint64_t ts_start;
};

// BPF program's internal hash map to save sched_waking event info
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 1024);
	__type(key, int);
	__type(value, struct sched_time);
} sched_waking_map SEC(".maps");

// BPF program's internal hash map to save sched_switch event info
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 1024);
	__type(key, int);
	__type(value, struct sched_time);
} sched_switch_map SEC(".maps");

// task_struct::state renamed to __state since v5.14
// Refer to https://github.com/torvalds/linux/commit/2f064a59a1 for more info
#define TASK_RUNNING	0x0

struct task_struct___o {
	volatile long int state;
} __attribute__((preserve_access_index));

struct task_struct___x {
	unsigned int __state;
} __attribute__((preserve_access_index));

static __always_inline __s64 get_task_state(void *task)
{
	struct task_struct___x *t = task;

	if (bpf_core_field_exists(t->__state))
		return BPF_CORE_READ(t, __state);
	return BPF_CORE_READ((struct task_struct___o *)task, state);
}

// sched_waking tracepoint
SEC("tp_btf/sched_waking")
int BPF_PROG(sched_waking, struct task_struct *task)
{
	uint64_t now = bpf_ktime_get_ns();
	uint8_t *filtered;
	pid_t pid;
	struct sched_time data;

	BPF_CORE_READ_INTO(&pid, task, pid);
	filtered = bpf_map_lookup_elem(&pid_filter_map, &pid);
	if (!filtered) {
		return 0;
	}

	data.ts_wakeup = now;
	bpf_map_update_elem(&sched_waking_map, &pid, &data, BPF_ANY);

	return 0;
}

// sched_switch tracepoint
SEC("tp_btf/sched_switch")
int BPF_PROG(sched_switch, bool preempt, struct task_struct *prev, struct task_struct *next)
{
	uint64_t now = bpf_ktime_get_ns();
	struct event *e;
	uint8_t *filtered;
	struct sched_time *pdata;
	pid_t pid;

	// Check if task is filtered and scheduled out
	BPF_CORE_READ_INTO(&pid, prev, pid);
	pdata = bpf_map_lookup_elem(&sched_switch_map, &pid);
	if (pdata) {
		struct schedstat_event *e;
		e = bpf_ringbuf_reserve(&buffer, sizeof(struct schedstat_event), 0);
		if (!e) {
			return 1;
		}

		e->pid = pid;
		e->ts_wakeup = pdata->ts_wakeup;
		e->ts_start = pdata->ts_start;
		e->ts_stop = now;
		e->cpu = bpf_get_smp_processor_id();
		bpf_ringbuf_submit(e, 0);

		// Check whether prev task is being preempted or not
		if (get_task_state(prev) == TASK_RUNNING) {
			// On being preempted, add a new entry into sched_wakeup_map
			struct sched_time data;

			data.ts_wakeup = pdata->ts_wakeup;
			bpf_map_update_elem(&sched_waking_map, &pid, &data, BPF_ANY);
		}

		bpf_map_delete_elem(&sched_switch_map, &pid);
	}

	// Check if task is filtered and scheduled in for execution
	BPF_CORE_READ_INTO(&pid, next, pid);
	filtered = bpf_map_lookup_elem(&pid_filter_map, &pid);
	if (filtered) {
		pdata = bpf_map_lookup_elem(&sched_waking_map, &pid);
		if (pdata) {
			struct sched_time data;

			data.ts_start = now;
			data.ts_wakeup = pdata->ts_wakeup;
			bpf_map_update_elem(&sched_switch_map, &pid, &data, BPF_ANY);

			bpf_map_delete_elem(&sched_waking_map, &pid);
		}
	}

	return 0;
}

char LICENSE[] SEC("license") = "GPL";
