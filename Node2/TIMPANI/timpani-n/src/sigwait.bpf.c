/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Common types for bpf and user-space programs
typedef __u8	uint8_t;
typedef __u16	uint16_t;
typedef __u32	uint32_t;
typedef __u64	uint64_t;
typedef __s8	int8_t;
typedef __s16	int16_t;
typedef __s32	int32_t;
typedef __s64	int64_t;

typedef int	pid_t;
typedef __u64	size_t;

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

struct sys_enter_rt_sigtimedwait {
	__u64 syscall_hdr;

	__u32 syscall_nr;
	void *uthese;
	void *uinfo;
	void *uts;
	size_t sigsetsize;
};

struct sys_exit_rt_sigtimedwait {
	__u64 syscall_hdr;

	__u32 syscall_nr;
	__u64 ret;
};

SEC("tracepoint/syscalls/sys_enter_rt_sigtimedwait")
int handle_sys_enter_rt_sigtimedwait(struct sys_enter_rt_sigtimedwait *ctx)
{
	__u64 now = bpf_ktime_get_ns();
	__u8 *filtered;
	__u64 id;
	pid_t pid;  // thread id in kernel, tid in user-space
	pid_t tgid; // thread group id (tgid) in kernel, pid in user-space

	id = bpf_get_current_pid_tgid();
	pid = (pid_t)id;
	tgid = id >> 32;

	filtered = bpf_map_lookup_elem(&pid_filter_map, &pid);
	if (filtered) {
		struct sigwait_event *e;

		e = bpf_ringbuf_reserve(&buffer, sizeof(struct sigwait_event), 0);
		if (!e) {
			return 1;
		}

		e->pid = pid;
		e->tgid = tgid;
		e->timestamp = now;
		e->enter = 1;

		bpf_ringbuf_submit(e, 0);
	}

	return 0;
}

SEC("tracepoint/syscalls/sys_exit_rt_sigtimedwait")
int handle_sys_exit_rt_sigtimedwait(struct sys_enter_rt_sigtimedwait *ctx)
{
	__u64 now = bpf_ktime_get_ns();
	__u8 *filtered;
	__u64 id;
	pid_t pid;  // thread id in kernel, tid in user-space
	pid_t tgid; // thread group id (tgid) in kernel, pid in user-space

	id = bpf_get_current_pid_tgid();
	pid = (pid_t)id;
	tgid = id >> 32;

	filtered = bpf_map_lookup_elem(&pid_filter_map, &pid);
	if (filtered) {
		struct sigwait_event *e;

		e = bpf_ringbuf_reserve(&buffer, sizeof(struct sigwait_event), 0);
		if (!e) {
			return 1;
		}

		e->pid = pid;
		e->tgid = tgid;
		e->timestamp = now;
		e->enter = 0;

		bpf_ringbuf_submit(e, 0);
	}

	return 0;
}
