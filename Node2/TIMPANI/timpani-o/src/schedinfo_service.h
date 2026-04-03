/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#ifndef SCHEDINFO_SERVICE_H
#define SCHEDINFO_SERVICE_H

#include <map>
#include <memory>
#include <shared_mutex>
#include <thread>
#include <grpcpp/grpcpp.h>

#include "proto/schedinfo.grpc.pb.h"
#include "node_config.h"  // Include NodeConfigManager
#include "global_scheduler.h"  // Include GlobalScheduler
#include "hyperperiod_manager.h"  // Include HyperperiodManager

using grpc::Server;
using grpc::ServerBuilder;
using grpc::ServerContext;
using grpc::Status;
using schedinfo::v1::Response;
using schedinfo::v1::SchedInfo;
using schedinfo::v1::SchedInfoService;
using schedinfo::v1::SchedPolicy;
using schedinfo::v1::TaskInfo;

using NodeSchedInfoMap = std::map<std::string, sched_info_t>;
using SchedInfoMap = std::map<std::string, NodeSchedInfoMap>;

/**
* @brief Implementation of the SchedInfoService gRPC service
*
* This service handles scheduling information deliveries from Pullpiri to Timpani-O.
* It processes SchedInfo messages and returns a Response indicating success or failure.
*/
class SchedInfoServiceImpl final : public SchedInfoService::Service
{
  public:
    explicit SchedInfoServiceImpl(std::shared_ptr<NodeConfigManager> node_config_manager = nullptr);

    Status AddSchedInfo(ServerContext* context, const SchedInfo* request,
                        Response* reply) override;

    SchedInfoMap GetSchedInfoMap(bool* changed = nullptr);

    /**
     * @brief Get hyperperiod information for a specific workload
     * @param workload_id The workload identifier
     * @return Pointer to HyperperiodInfo or nullptr if not found
     */
    const HyperperiodInfo* GetHyperperiodInfo(const std::string& workload_id) const;

    /**
     * @brief Get all hyperperiod information
     * @return Map of workload_id to HyperperiodInfo
     */
    const std::map<std::string, HyperperiodInfo>& GetAllHyperperiods() const;

  private:
    static int SchedPolicyToInt(SchedPolicy policy);

    // Convert gRPC TaskInfo to internal Task structure
    std::vector<Task> ConvertTaskInfoToTasks(const SchedInfo* request);

    // Member variable to store scheduling information
    SchedInfoMap sched_info_map_;
    // Use shared_mutex for sched_info_map_
    mutable std::shared_mutex sched_info_mutex_;
    // Flag for DBusServer indicating whether sched_info_map_ has changed
    bool sched_info_changed_;
    // Node configuration manager
    std::shared_ptr<NodeConfigManager> node_config_manager_;
    // Global scheduler
    std::shared_ptr<GlobalScheduler> global_scheduler_;
    // Hyperperiod manager
    std::shared_ptr<HyperperiodManager> hyperperiod_manager_;
};

/**
 * @brief SchedInfoServer class for managing the SchedInfoService gRPC server
 *
 * This class encapsulates the SchedInfoService gRPC server functionality,
 * allowing it to start and stop the service, and handle incoming requests.
 */
class SchedInfoServer
{
  public:
    explicit SchedInfoServer(std::shared_ptr<NodeConfigManager> node_config_manager = nullptr);
    ~SchedInfoServer();
    bool Start(int port);
    void Stop();
    SchedInfoMap GetSchedInfoMap(bool* changed = nullptr);

    /**
     * @brief Get hyperperiod information for a specific workload
     * @param workload_id The workload identifier
     * @return Pointer to HyperperiodInfo or nullptr if not found
     */
    const HyperperiodInfo* GetHyperperiodInfo(const std::string& workload_id) const;

    /**
     * @brief Get all hyperperiod information
     * @return Map of workload_id to HyperperiodInfo
     */
    const std::map<std::string, HyperperiodInfo>& GetAllHyperperiods() const;

    void DumpSchedInfo();

 private:
    SchedInfoServiceImpl service_;
    std::unique_ptr<Server> server_;
    std::unique_ptr<std::thread> server_thread_;
};

#endif  // SCHEDINFO_SERVICE_H
