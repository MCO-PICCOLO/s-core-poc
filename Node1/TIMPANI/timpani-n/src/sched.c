/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#define _GNU_SOURCE
#include "internal.h"
#include <sched.h>
#include <dirent.h>
#include <sys/syscall.h>
#include <signal.h>

// Ensure SCHED_NORMAL is defined (sometimes defined as SCHED_OTHER)
#ifndef SCHED_NORMAL
#define SCHED_NORMAL 0
#endif
#define PROCESS_NAME_SIZE	16

ttsched_error_t set_affinity(pid_t pid, int cpu) {
	cpu_set_t cpuset;
	int num_cpus = sysconf(_SC_NPROCESSORS_ONLN);

	if (num_cpus < 0) {
		TT_LOG_ERROR("Failed to get number of CPUs: %s", strerror(errno));
		return TTSCHED_ERROR_SYSTEM;
	}

	// Validate CPU number
	if (cpu < 0 || cpu >= num_cpus) {
		TT_LOG_WARNING("Invalid CPU %d (available: 0-%d), setting to CPU 0",
			cpu, num_cpus - 1);
		cpu = 0; // Fallback to CPU 0
	}

	CPU_ZERO(&cpuset);
	CPU_SET(cpu, &cpuset);

	// Set pid's CPU affinity mask
	if (sched_setaffinity(pid, sizeof(cpu_set_t), &cpuset) == -1) {
		TT_LOG_ERROR("sched_setaffinity failed for PID %d with CPU %d: %s",
			pid, cpu, strerror(errno));
		return TTSCHED_ERROR_PERMISSION;
	}

	TT_LOG_INFO("Successfully set CPU affinity for PID %d to CPU %d", pid, cpu);
	return TTSCHED_SUCCESS;
}

// Sets CPU affinity for the given pid using a bitmask.
// Each bit in cpumask represents a CPU core (bit 0 = CPU 0, bit 1 = CPU 1, ...).
ttsched_error_t set_affinity_cpumask(pid_t pid, uint64_t cpumask) {
	cpu_set_t cpuset;

	CPU_ZERO(&cpuset);
	for (int i = 0; i < sizeof(cpumask) * 8; ++i) {
		if (cpumask & (1ULL << i)) {
			CPU_SET(i, &cpuset);
		}
	}

	// Set pid's CPU affinity mask
	if (sched_setaffinity(pid, sizeof(cpu_set_t), &cpuset) < 0) {
		TT_LOG_ERROR("sched_setaffinity failed for PID %d with cpumask 0x%lx: %s",
			pid, cpumask, strerror(errno));
		return TTSCHED_ERROR_PERMISSION;
	}

	return TTSCHED_SUCCESS;
}

// Sets CPU affinity for all threads in a process using a bitmask.
// Iterates through all threads in /proc/<pid>/task/ and applies the cpumask to each.
ttsched_error_t set_affinity_cpumask_all_threads(pid_t pid, uint64_t cpumask) {
	char task_path[256];
	int success_count = 0;
	int failure_count = 0;

	if (pid <= 0) {
		TT_LOG_ERROR("Invalid PID %d", pid);
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	snprintf(task_path, sizeof(task_path), "/proc/%d/task", pid);

	DIR *task_dir = opendir(task_path);
	if (!task_dir) {
		TT_LOG_ERROR("Failed to open %s: %s", task_path, strerror(errno));
		return TTSCHED_ERROR_SYSTEM;
	}

	struct dirent *entry;
	while ((entry = readdir(task_dir)) != NULL) {
		if (entry->d_type == DT_DIR) {
			int tid = atoi(entry->d_name);
			if (tid > 0) {  // Skip '.' and '..' and non-numeric entries
				ttsched_error_t ret = set_affinity_cpumask(tid, cpumask);
				if (ret == TTSCHED_SUCCESS) {
					success_count++;
					TT_LOG_DEBUG("Set affinity for thread %d (PID %d) to cpumask 0x%lx",
						tid, pid, cpumask);
				} else {
					failure_count++;
					TT_LOG_WARNING("Failed to set affinity for thread %d (PID %d): %s",
						tid, pid, ttsched_error_string(ret));
				}
			}
		}
	}
	closedir(task_dir);

	if (success_count == 0 && failure_count == 0) {
		TT_LOG_DEBUG("No threads found for PID %d", pid);
		return TTSCHED_SUCCESS;
	}

	TT_LOG_INFO("Set CPU affinity for %d threads in PID %d to cpumask 0x%lx (%d succeeded, %d failed)",
		success_count + failure_count, pid, cpumask, success_count, failure_count);

	// Return success if at least one thread was successfully configured
	return (success_count > 0) ? TTSCHED_SUCCESS : TTSCHED_ERROR_PERMISSION;
}

static int set_sched_attr_syscall(pid_t pid, const struct sched_attr_tt *attr,
			unsigned int flags)
{
	return syscall(SYS_sched_setattr, pid, attr, flags);
}

ttsched_error_t set_schedattr(pid_t pid, unsigned int priority, unsigned int policy) {
	struct sched_attr_tt attr;

	// 입력 인자 검증
	if (priority > 99) {
		TT_LOG_ERROR("Invalid priority %u (must be <= 99)", priority);
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	if (policy != SCHED_NORMAL && policy != SCHED_FIFO && policy != SCHED_RR) {
		TT_LOG_ERROR("Invalid policy %u", policy);
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	memset(&attr, 0, sizeof(attr));
	attr.size = sizeof(struct sched_attr_tt);
	attr.sched_priority = priority;
	attr.sched_policy = policy;

	if (set_sched_attr_syscall(pid, &attr, 0) == -1) {
		TT_LOG_ERROR("sched_setattr failed for PID %d: %s", pid, strerror(errno));
		return TTSCHED_ERROR_PERMISSION;
	}
	TT_LOG_INFO("Successfully set scheduling attributes for PID %d (priority=%u, policy=%u)",
		pid, priority, policy);
	return TTSCHED_SUCCESS;
}

ttsched_error_t get_process_name_by_pid(const int pid, char name[])
{
	if (!name) {
		TT_LOG_ERROR("Invalid name buffer pointer");
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	if (pid <= 0) {
		TT_LOG_ERROR("Invalid PID %d", pid);
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	char procpath[60] = {};
	sprintf(procpath, "/proc/%d/comm", pid);

	FILE* f = fopen(procpath, "r");
	if (!f) {
		TT_LOG_ERROR("Failed to open %s: %s", procpath, strerror(errno));
		return TTSCHED_ERROR_SYSTEM;
	}

	size_t size = fread(name, sizeof(char), PROCESS_NAME_SIZE, f);
	if (size > 0) {
		if ('\n' == name[size-1])
			name[size-1] = '\0';
	}
	fclose(f);

	return TTSCHED_SUCCESS;
}

static void get_thread_name(pid_t pid, pid_t tid, char *name, size_t len)
{
	char path[256];
	snprintf(path, sizeof(path), "/proc/%d/task/%d/comm", pid, tid);

	FILE *file = fopen(path, "r");
	if (file == NULL) {
		return;
	}

	if (fgets(name, len, file) == NULL) {
		name[0] = '\0';  // 실패 시 빈 문자열
	}
	fclose(file);

	// Remove the newline character at the end
	size_t nl = strcspn(name, "\n");
	if (name[nl] == '\n') {
		name[nl] = '\0';
	}
}

static int find_threads_by_name(const char *name, int pid)
{
	int ret = -1;
	char path[256];
	snprintf(path, sizeof(path), "/proc/%d/task", pid);

	DIR *dir = opendir(path);
	if (!dir) {
		perror("opendir");
		return -1;
	}

	struct dirent *entry;
	while ((entry = readdir(dir)) != NULL) {
		if (entry->d_type == DT_DIR) {
			int tid = atoi(entry->d_name);
			if (tid > 0) {	// Skip '.' and '..' and non-numeric entries
				char tname[256];
				get_thread_name(pid, tid, tname, sizeof(tname));
				if (strcmp(name, tname) == 0) {
					// found it
					ret = tid;
					break;
				}
			}
		}
	}
	closedir(dir);
	return ret;
}

ttsched_error_t get_pid_by_name(const char *name, int *pid)
{
	if (!name || !pid) {
		TT_LOG_ERROR("Invalid name or pid pointer");
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	*pid = -1;

	DIR *proc_dir = opendir("/proc");
	if (!proc_dir) {
		TT_LOG_ERROR("Failed to open /proc: %s", strerror(errno));
		return TTSCHED_ERROR_SYSTEM;
	}

	struct dirent *entry;
	while ((entry = readdir(proc_dir)) != NULL) {
		if (entry->d_type == DT_DIR) {
			int current_pid = atoi(entry->d_name);
			if (current_pid > 0) {	// Skip '.' and '..' and non-numeric entries
				int tid = find_threads_by_name(name, current_pid);
				if (tid != -1) {
					*pid = tid;
					break;
				}
			}
		}
	}
	closedir(proc_dir);

	if (*pid == -1) {
		TT_LOG_WARNING("Process with name '%s' not found", name);
		return TTSCHED_ERROR_SYSTEM;
	}

	return TTSCHED_SUCCESS;
}

static int _get_pid_by_nspid(int current_pid, const char *name, int nspid)
{
	char path[PATH_MAX];
	int pid;

	snprintf(path, sizeof(path), "/proc/%d/status", current_pid);
	FILE *fp = fopen(path, "r");
	if (!fp) return -1;

	char line[256];
	while (fgets(line, sizeof(line), fp)) {
		if (strncmp(line, "Name:", 5) == 0) {
			// Get this line: "Name:   process_name"
			char proc_name[PROCESS_NAME_SIZE];
			sscanf(line + 5, "%15s", proc_name);
			if (strncmp(proc_name, name, 15) != 0) {
				// Name does not match
				break;
			}
		} else if (strncmp(line, "NSpid:", 6) == 0) {
			// Get this line: "NSpid:  12345  100"
			char *saveptr;
			char *token = strtok_r(line + 6, " \t\n", &saveptr);
			if (token) {
				pid = atoi(token);
				token = strtok_r(NULL, " \t\n", &saveptr);
				if (token) {
					if (atoi(token) == nspid) {
						// Found matching nspid
						fclose(fp);
						return pid;
					}
				}
			}
			// No matching nspid found
			break;
		}
	}
	fclose(fp);
	return -1;
}

ttsched_error_t get_pid_by_nspid(const char *name, int nspid, int *pid)
{
	if (!name || !pid) {
		TT_LOG_ERROR("Invalid name, pid pointer");
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	*pid = -1;

	DIR *proc_dir = opendir("/proc");
	if (!proc_dir) {
		TT_LOG_ERROR("Failed to open /proc: %s", strerror(errno));
		return TTSCHED_ERROR_SYSTEM;
	}

	struct dirent *entry;
	while ((entry = readdir(proc_dir)) != NULL) {
		if (entry->d_type == DT_DIR) {
			int current_pid = atoi(entry->d_name);
			if (current_pid > 0) {	// Skip '.' and '..' and non-numeric entries
				*pid = _get_pid_by_nspid(current_pid, name, nspid);
				if (*pid != -1) {
					break;
				}
			}
		}
	}
	closedir(proc_dir);

	if (*pid == -1) {
		TT_LOG_DEBUG("Process with name '%s' and nspid %d not found", name, nspid);
		return TTSCHED_ERROR_SYSTEM;
	}

	return TTSCHED_SUCCESS;
}

static int open_pidfd_syscall(pid_t pid, unsigned int flags)
{
	return syscall(SYS_pidfd_open, pid, flags);
}

static int send_signal_pidfd_syscall(int pidfd, int sig, siginfo_t *info, unsigned int flags)
{
	return syscall(SYS_pidfd_send_signal, pidfd, sig, info, flags);
}

ttsched_error_t create_pidfd(pid_t pid, int *pidfd)
{
	if (!pidfd) {
		TT_LOG_ERROR("Invalid pidfd pointer");
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	if (pid <= 0) {
		TT_LOG_ERROR("Invalid PID %d", pid);
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	*pidfd = open_pidfd_syscall(pid, 0);
	if (*pidfd < 0) {
		TT_LOG_ERROR("pidfd_open failed for PID %d: %s", pid, strerror(errno));
		return TTSCHED_ERROR_PERMISSION;
	}
	return TTSCHED_SUCCESS;
}

ttsched_error_t send_signal_pidfd(int pidfd, int signal)
{
	if (pidfd < 0) {
		TT_LOG_ERROR("Invalid pidfd %d", pidfd);
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	int ret = send_signal_pidfd_syscall(pidfd, signal, NULL, 0);
	if (ret < 0) {
		int saved_errno = errno;
		TT_LOG_ERROR("pidfd_send_signal failed: %s", strerror(saved_errno));
		if (saved_errno == ESRCH)
			return TTSCHED_ERROR_NO_PROCESS;
		if (saved_errno == EPERM)
			return TTSCHED_ERROR_PERMISSION;
		return TTSCHED_ERROR_SYSTEM;
	}
	return TTSCHED_SUCCESS;
}

ttsched_error_t is_process_alive(int pidfd, int *alive)
{
	if (!alive) {
		TT_LOG_ERROR("Invalid alive pointer");
		return TTSCHED_ERROR_INVALID_ARGS;
	}

	if (pidfd < 0) {
		*alive = 0;
		return TTSCHED_SUCCESS;
	}

	// Try a null signal to check if process is alive
	int ret = send_signal_pidfd_syscall(pidfd, 0, NULL, 0);
	*alive = (ret == 0) ? 1 : 0;
	return TTSCHED_SUCCESS;
}
