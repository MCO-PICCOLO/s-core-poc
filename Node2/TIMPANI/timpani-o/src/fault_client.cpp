/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <iostream>

#include "tlog.h"
#include "fault_client.h"

FaultServiceClient& FaultServiceClient::GetInstance()
{
    static FaultServiceClient instance;
    return instance;

}

bool FaultServiceClient::Initialize(const std::string& server_address)
{
    if (initialized_) {
        TLOG_WARN("FaultServiceClient already initialized");
        return true;
    }

    if (server_address.empty()) {
        TLOG_ERROR("Server address cannot be empty");
        return false;
    }

    if (!CreateChannel(server_address)) {
        TLOG_ERROR("Failed to create gRPC channel to Pullpiri");
        return false;
    }

    initialized_ = true;
    return true;
}

bool FaultServiceClient::IsInitialized() const
{
    return initialized_;
}

bool FaultServiceClient::NotifyFault(const std::string &workload_id,
                                     const std::string &node_id,
                                     const std::string &task_name,
                                     FaultType fault_type)
{
    if (!initialized_) {
        TLOG_ERROR("FaultServiceClient not initialized");
        return false;
    }

    FaultInfo request;
    request.set_workload_id(workload_id);
    request.set_node_id(node_id);
    request.set_task_name(task_name);
    request.set_type(fault_type);

    Response reply;
    ClientContext context;

    TLOG_INFO("Notifying Pullpiri - Workload: ", workload_id,
              ", Node: ", node_id, ", Task: ", task_name,
              ", Fault Type: ", FaultTypeToStr(fault_type));

    Status status = stub_->NotifyFault(&context, request, &reply);

    if (!status.ok()) {
        TLOG_ERROR("NotifyFault failed: ", status.error_code(), ": ",
                   status.error_message());
        return false;
    }

    if (reply.status() != 0) {
        TLOG_ERROR("NotifyFault: Pullpiri returned error: ", reply.status());
        return false;
    }

    return true;
}

FaultServiceClient::FaultServiceClient()
    : initialized_(false)
{
}

FaultServiceClient::~FaultServiceClient()
{
    if (channel_) {
        // gRPC channel and stub will be cleaned up automatically
    }
}

bool FaultServiceClient::CreateChannel(const std::string& server_address)
{
    try {
        // Create gRPC channel with insecure credentials for now
        // TODO: Consider using secure credentials in production
        channel_ = grpc::CreateChannel(server_address,
                                       grpc::InsecureChannelCredentials());
        if (!channel_) {
            TLOG_ERROR("grpc::CreateChannel failed");
            return false;
        }

        // Create the stub
        stub_ = FaultService::NewStub(channel_);
        if (!stub_) {
            TLOG_ERROR("FaultService::NewStub failed");
            return false;
        }

        return true;
    }
    catch (const std::exception& e) {
        TLOG_ERROR("Exception while creating gRPC channel: ", e.what());
        return false;
    }
}

const char* FaultServiceClient::FaultTypeToStr(FaultType type)
{
    switch (type) {
        case FaultType::UNKNOWN:
            return "UNKNOWN";
        case FaultType::DMISS:
            return "DMISS";
        default:
            return "INVALID";
    }
}
