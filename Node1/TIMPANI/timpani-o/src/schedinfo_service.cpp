/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include "tlog.h"
#include "schedinfo_service.h"
#include "dbus_server.h"

SchedInfoServiceImpl::SchedInfoServiceImpl(std::shared_ptr<NodeConfigManager> node_config_manager)
    : node_config_manager_(node_config_manager),
      sched_info_changed_(false)
{
    TLOG_INFO("SchedInfoServiceImpl created with GlobalScheduler and HyperperiodManager integration");

    // Create GlobalScheduler instance
    global_scheduler_ = std::make_shared<GlobalScheduler>(node_config_manager_);

    // Create HyperperiodManager instance
    hyperperiod_manager_ = std::make_shared<HyperperiodManager>();

    if (node_config_manager_ && node_config_manager_->IsLoaded()) {
        TLOG_INFO("Node configuration loaded with ", node_config_manager_->GetAllNodes().size(), " nodes");
    } else {
        TLOG_INFO("Using default node configuration");
    }
}

Status SchedInfoServiceImpl::AddSchedInfo(ServerContext* context,
                                          const SchedInfo* request,
                                          Response* reply)
{
    TLOG_INFO("Received SchedInfo: ", request->workload_id(), " with ",
              request->tasks_size(), " tasks");

    // Print detailed task information
    for (int i = 0; i < request->tasks_size(); i++) {
        const auto& task = request->tasks(i);
        TLOG_DEBUG("Task ", i, ": ", task.name());
        TLOG_DEBUG("  Priority: ", task.priority());
        TLOG_DEBUG("  Policy: ", task.policy());
        TLOG_DEBUG("  CPU Affinity: 0x", std::setfill('0'), std::setw(16),
                   std::hex, task.cpu_affinity(), std::dec);
        TLOG_DEBUG("  Period: ", task.period());
        TLOG_DEBUG("  Runtime: ", task.runtime());
        TLOG_DEBUG("  Deadline: ", task.deadline());
        TLOG_DEBUG("  Release Time: ", task.release_time());
        TLOG_DEBUG("  Max Deadline Misses: ", task.max_dmiss());
        TLOG_DEBUG("  Node ID: ", task.node_id());
    }

    std::unique_lock<std::shared_mutex> lock(sched_info_mutex_);

    // Currently only supports one workload at a time
    // Clear previous workload if exists to support workload replacement
    if (!sched_info_map_.empty()) {
        std::string prev_workload = sched_info_map_.begin()->first;
        TLOG_WARN("Replacing existing workload '", prev_workload, "' with new workload '", request->workload_id(), "'");

        // Clean up Apex.OS allocated memory for scheduling info
        const std::string& workload_id = sched_info_map_.begin()->first;
        if (workload_id == "Apex.OS") {
            auto& node_sched_map = sched_info_map_.begin()->second;
            for (const auto& node_entry : node_sched_map) {
                const sched_info_t& sched_info = node_entry.second;
                if (sched_info.tasks != nullptr) {
                    free(sched_info.tasks);
                }
            }
            node_sched_map.clear();
        }
        // Clear existing scheduling info
        sched_info_map_.clear();
        // Clear global scheduler state when replacing workload
        global_scheduler_->clear();
        // Clear hyperperiod information for previous workload
        hyperperiod_manager_->ClearWorkload(prev_workload);

        sched_info_changed_ = true;
        // Proactively invalidate the cached D-Bus buffer so that a
        // restarted timpani-n always receives the new schedule.
        DBusServer::GetInstance().FreeSchedInfoBuf();
    }

    // Special handling for Apex.OS workload
    if (request->workload_id() == "Apex.OS") {
        TLOG_INFO("Skipping GlobalScheduler for workload: ", request->workload_id());

        // Group tasks by node_id first
        std::map<std::string, std::vector<sched_task_t>> node_tasks_map;

        for (const auto& grpc_task : request->tasks()) {
            TLOG_INFO("Timpani: Processing task: ", grpc_task.name(),
                   " on node: ", grpc_task.node_id(),
                   " with priority: ", grpc_task.priority(),
                   " and period: ", grpc_task.period(), "us");

            const std::string& node_id = grpc_task.node_id();

            sched_task_t task;
            memset(&task, 0, sizeof(sched_task_t));
            std::strncpy(task.task_name, grpc_task.name().c_str(), sizeof(task.task_name) - 1);
            task.task_name[sizeof(task.task_name) - 1] = '\0';
            std::strncpy(task.assigned_node, node_id.c_str(), sizeof(task.assigned_node) - 1);
            task.assigned_node[sizeof(task.assigned_node) - 1] = '\0';
            task.cpu_affinity = grpc_task.cpu_affinity();
            task.period_ns = static_cast<uint64_t>(grpc_task.period()) * 1000;
            task.max_dmiss = grpc_task.max_dmiss();

            node_tasks_map[node_id].push_back(task);
        }

        // Construct node_sched_map with proper memory allocation
        NodeSchedInfoMap node_sched_map;

        for (const auto& node_entry : node_tasks_map) {
            const std::string& node_id = node_entry.first;
            const auto& tasks = node_entry.second;

            node_sched_map[node_id].num_tasks = tasks.size();
            node_sched_map[node_id].tasks = (sched_task_t*)malloc(sizeof(sched_task_t) * tasks.size());

            for (int i = 0; i < tasks.size(); i++) {
                node_sched_map[node_id].tasks[i] = tasks[i];
            }
        }

        // Store in our sched_info_map_ (copy the results)
        sched_info_map_[request->workload_id()] = node_sched_map;

        reply->set_status(0);  // Success
        return Status::OK;
    }

    // Convert gRPC TaskInfo to internal Task structures
    std::vector<Task> tasks = ConvertTaskInfoToTasks(request);

    // Calculate hyperperiod for this workload BEFORE scheduling
    uint64_t hyperperiod = hyperperiod_manager_->CalculateHyperperiod(request->workload_id(), tasks);
    if (hyperperiod == 0) {
        TLOG_ERROR("Failed to calculate hyperperiod for workload: ", request->workload_id());
        reply->set_status(-1);  // Indicate failure
        return Status::OK;
    }

    // Use GlobalScheduler to process tasks
    global_scheduler_->clear();
    global_scheduler_->set_tasks(tasks);

    // Validate workload distribution across nodes
    std::map<std::string, int> node_task_counts;
    for (const auto& task : tasks) {
        node_task_counts[task.target_node]++;
    }

    TLOG_INFO("Workload '", request->workload_id(), "' distributes tasks across ",
              node_task_counts.size(), " nodes:");
    for (const auto& entry : node_task_counts) {
        TLOG_INFO("  Node '", entry.first, "': ", entry.second, " tasks");
    }

    // Execute scheduling algorithm with new target node priority logic
    bool scheduling_success = global_scheduler_->schedule("target_node_priority");
    if (!scheduling_success) {
        TLOG_ERROR("Scheduling failed for workload: ", request->workload_id());
        reply->set_status(-1);  // Indicate failure
        return Status::OK;
    }

    // Get scheduled results from GlobalScheduler
    const NodeSchedInfoMap& node_sched_map = global_scheduler_->get_sched_info_map();

    // Verify the structure: NodeSchedInfoMap is std::map<std::string, sched_info_t>
    // where key is node_id and value is sched_info_t containing tasks for that node
    TLOG_INFO("Generated schedules for ", node_sched_map.size(), " nodes:");
    for (const auto& entry : node_sched_map) {
        const std::string& node_id = entry.first;
        const sched_info_t& sched_info = entry.second;
        TLOG_INFO("  Node '", node_id, "': ", sched_info.num_tasks, " tasks");
    }

    // Store in our sched_info_map_ (copy the results)
    // sched_info_map_[workload_id] = NodeSchedInfoMap (node_id -> sched_info_t)
    sched_info_map_[request->workload_id()] = node_sched_map;

    TLOG_INFO("Successfully scheduled ", global_scheduler_->get_total_scheduled_tasks(),
            " tasks across ", node_sched_map.size(), " nodes");
    TLOG_INFO("Hyperperiod for workload '", request->workload_id(), "': ",
              hyperperiod, " us (", hyperperiod / 1000, " ms)");
    reply->set_status(0);  // Success
    return Status::OK;
}

std::vector<Task> SchedInfoServiceImpl::ConvertTaskInfoToTasks(const SchedInfo* request)
{
    std::vector<Task> tasks;
    tasks.reserve(request->tasks_size());

    for (int i = 0; i < request->tasks_size(); i++) {
        const auto& grpc_task = request->tasks(i);

        Task task;
        task.name = grpc_task.name();
        task.workload_id = request->workload_id();  // Set workload_id from request
        task.policy = SchedPolicyToInt(static_cast<SchedPolicy>(grpc_task.policy()));
        task.priority = grpc_task.priority();
        task.cpu_affinity = grpc_task.cpu_affinity();
        task.period_us = grpc_task.period();
        task.runtime_us = grpc_task.runtime();
        task.deadline_us = grpc_task.deadline();
        task.release_time = grpc_task.release_time();
        task.max_dmiss = grpc_task.max_dmiss();
        task.target_node = grpc_task.node_id();

        // Convert CPU affinity bitmask to a comma-separated list of allowed CPU IDs.
        // All set bits are enumerated so the scheduler can pick the best CPU
        // among them and timpani-n receives the full multi-bit mask.
        if (grpc_task.cpu_affinity() == 0xFFFFFFFF || grpc_task.cpu_affinity() == 0) {
            task.affinity = "any";
        } else {
            uint32_t affinity_mask = static_cast<uint32_t>(grpc_task.cpu_affinity());
            std::string affinity_str;
            for (int bit = 0; bit < 32; ++bit) {
                if (affinity_mask & (1u << bit)) {
                    if (!affinity_str.empty()) affinity_str += ",";
                    affinity_str += std::to_string(bit);
                }
            }
            task.affinity = affinity_str;
        }

        // Set reasonable defaults for other fields
        task.memory_mb = 256;  // Default memory requirement
        task.assigned_node = "";  // Will be set by scheduler
        task.assigned_cpu = -1;   // Will be set by scheduler

        tasks.push_back(task);
    }

    return tasks;
}

SchedInfoMap SchedInfoServiceImpl::GetSchedInfoMap(bool* changed)
{
    std::shared_lock<std::shared_mutex> lock(sched_info_mutex_);
    if (changed) {
        // Reset the sched_info_changed_ flag after reading the value
        *changed = sched_info_changed_;
        sched_info_changed_ = false;
    }
    return sched_info_map_;
}

const HyperperiodInfo* SchedInfoServiceImpl::GetHyperperiodInfo(const std::string& workload_id) const
{
    return hyperperiod_manager_->GetHyperperiodInfo(workload_id);
}

const std::map<std::string, HyperperiodInfo>& SchedInfoServiceImpl::GetAllHyperperiods() const
{
    return hyperperiod_manager_->GetAllHyperperiods();
}

int SchedInfoServiceImpl::SchedPolicyToInt(SchedPolicy policy)
{
    switch (policy) {
        case SchedPolicy::NORMAL:
            return 0;
        case SchedPolicy::FIFO:
            return 1;
        case SchedPolicy::RR:
            return 2;
        default:
            return -1;
    }
}

SchedInfoServer::SchedInfoServer(std::shared_ptr<NodeConfigManager> node_config_manager)
	: service_(node_config_manager), server_(nullptr), server_thread_(nullptr)
{
    TLOG_INFO("SchedInfoServer created with node configuration");
}

SchedInfoServer::~SchedInfoServer() { Stop(); }

bool SchedInfoServer::Start(int port)
{
    std::string server_addr = "0.0.0.0";
    server_addr += ":" + std::to_string(port);

    ServerBuilder builder;
    builder.AddListeningPort(server_addr, grpc::InsecureServerCredentials());
    builder.AddChannelArgument(GRPC_ARG_ALLOW_REUSEPORT, 0);
    builder.RegisterService(&service_);

    server_ = builder.BuildAndStart();
    if (!server_) {
        TLOG_ERROR("Failed to start SchedInfoService on ", server_addr);
        return false;
    }

    // Start the server in a separate thread
    server_thread_ =
        std::make_unique<std::thread>([this]() { server_->Wait(); });

    return true;
}

void SchedInfoServer::Stop()
{
    if (server_) {
        server_->Shutdown();
    }
    if (server_thread_ && server_thread_->joinable()) {
        server_thread_->join();
    }
}

SchedInfoMap SchedInfoServer::GetSchedInfoMap(bool* changed)
{
    return service_.GetSchedInfoMap(changed);
}

const HyperperiodInfo* SchedInfoServer::GetHyperperiodInfo(const std::string& workload_id) const
{
    return service_.GetHyperperiodInfo(workload_id);
}

const std::map<std::string, HyperperiodInfo>& SchedInfoServer::GetAllHyperperiods() const
{
    return service_.GetAllHyperperiods();
}

void SchedInfoServer::DumpSchedInfo()
{
    const SchedInfoMap sched_info_map = service_.GetSchedInfoMap();

    if (sched_info_map.empty()) {
        TLOG_INFO("No schedule info available");
        return;
    }

    TLOG_INFO("Dumping SchedInfoMap:");
    for (const auto& entry : sched_info_map) {
        const std::string& workload_id = entry.first;
        const auto& node_sched_info = entry.second;

        TLOG_INFO("Workload ID: ", workload_id, " with ", node_sched_info.size(), " nodes");

        for (const auto& node : node_sched_info) {
            const std::string& node_id = node.first;
            const sched_info_t& schedule_info = node.second;

            TLOG_INFO("Node ID: ", node_id, " with ", schedule_info.num_tasks, " tasks");

            for (int i = 0; i < schedule_info.num_tasks; i++) {
                const sched_task_t& task = schedule_info.tasks[i];
                TLOG_DEBUG("  Task Name: ", task.task_name);
                TLOG_DEBUG("    Assigned Node: ", task.assigned_node);
                TLOG_DEBUG("    CPU Affinity: ", task.cpu_affinity);
                TLOG_DEBUG("    Priority: ", task.sched_priority);
                TLOG_DEBUG("    Policy: ", task.sched_policy);
                TLOG_DEBUG("    Period: ", task.period_ns / 1000000, "ms");
                TLOG_DEBUG("    Runtime: ", task.runtime_ns / 1000000, "ms");
                TLOG_DEBUG("    Deadline: ", task.deadline_ns / 1000000, "ms");
            }
        }
    }
}
