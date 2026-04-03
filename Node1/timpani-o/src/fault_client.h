#ifndef FAULT_CLIENT_H
#define FAULT_CLIENT_H

#include <memory>
#include <string>
#include <grpcpp/grpcpp.h>

#include "proto/schedinfo.grpc.pb.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using schedinfo::v1::FaultInfo;
using schedinfo::v1::FaultService;
using schedinfo::v1::FaultType;
using schedinfo::v1::Response;

/**
 * @brief Singleton gRPC client for communicating with Piccolo's FaultService
 *
 * This class implements the Singleton design pattern and manages a persistent
 * gRPC connection to Piccolo for fault notifications.
 *
 * Usage example:
 *   FaultServiceClient& client = FaultServiceClient::GetInstance();
 *   if (client.Initialize("localhost:50053")) {
 *       client.NotifyFault("workload_1", "node_1", "task_1", FaultType::DMISS);
 *   }
 */
class FaultServiceClient
{
  public:
    static FaultServiceClient& GetInstance();

    // Initialize the gRPC connection to Piccolo
    bool Initialize(const std::string& server_address);

    // Check if the client is properly initialized
    bool IsInitialized() const;

    bool NotifyFault(const std::string &workload_id,
                     const std::string &node_id,
                     const std::string &task_name,
                     FaultType fault_type);

  private:
    FaultServiceClient();
    FaultServiceClient(const FaultServiceClient&) = delete;
    FaultServiceClient& operator=(const FaultServiceClient&) = delete;
    ~FaultServiceClient();

    static const char* FaultTypeToStr(FaultType type);

    // Create gRPC channel and stub
    bool CreateChannel(const std::string& server_address);

    std::shared_ptr<Channel> channel_;
    std::unique_ptr<FaultService::Stub> stub_;
    bool initialized_;
};

#endif  // FAULT_CLIENT_H
