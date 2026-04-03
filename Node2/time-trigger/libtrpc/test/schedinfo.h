#ifndef _SCHEDINFO_H_
#define _SCHEDINFO_H_

#ifdef __cplusplus
extern "C" {
#endif

struct task_info {
	uint32_t pid;
	char name[16];
	uint32_t period;
	uint32_t release_time;
	struct task_info *next;
};

struct sched_info {
	char container_id[64];
	int32_t container_rt_runtime;
	int32_t container_rt_period;
	uint64_t cpumask;

	int32_t container_period;
	int32_t pod_period;

	uint32_t nr_tasks;

	struct task_info *tasks;
};

#ifdef __cplusplus
}
#endif

#endif	/* _SCHEDINFO_H_ */
