/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <iostream>
#include <string>
#include <memory>
#include <chrono>
#include <thread>
#include <getopt.h>

#include "tlog.h"
#include "schedinfo_service.h"
#include "fault_client.h"
#include "dbus_server.h"
#include "node_config.h"

bool RunSchedInfoServer(int port, std::unique_ptr<SchedInfoServer>& server,
                        std::shared_ptr<NodeConfigManager> node_config_manager)
{
    server = std::make_unique<SchedInfoServer>(node_config_manager);
    if (!server->Start(port)) {
        TLOG_ERROR("Failed to start SchedInfoServer on port ", port);
        return false;
    }
    TLOG_INFO("SchedInfoServer listening on port ", port);
    return true;
}

bool InitFaultClient(const std::string& addr, int port)
{
    std::string piccolo_addr = addr + ":" + std::to_string(port);

    FaultServiceClient& client = FaultServiceClient::GetInstance();
    return client.Initialize(piccolo_addr);
}

bool NotifyFaultDemo()
{
    FaultServiceClient& client = FaultServiceClient::GetInstance();

    return client.NotifyFault("workload_demo", "node_demo", "task_demo", FaultType::DMISS);
}

bool RunDBusServer(int port, SchedInfoServer* sinfo_server)
{
    DBusServer& server = DBusServer::GetInstance();
    if (!server.Start(port, sinfo_server)) {
        TLOG_ERROR("Failed to start DBusServer on port ", port);
        return false;
    }
    TLOG_INFO("DBusServer listening on port ", port);
    return true;
}

bool GetOptions(int argc, char *argv[], int& sinfo_port,
                std::string& fault_addr, int& fault_port,
                int& dbus_port, bool& notify_fault, std::string& node_config_file)
{
    const char* short_opts = "hs:f:p:d:nc:";
    const struct option long_opts[] = {
        {"help", no_argument, nullptr, 'h'},
        {"sinfoport", required_argument, nullptr, 's'},
        {"faulthost", required_argument, nullptr, 'f'},
        {"faultport", required_argument, nullptr, 'p'},
        {"dbusport", required_argument, nullptr, 'd'},
        {"notifyfault", no_argument, nullptr, 'n'},
        {"node-config", required_argument, nullptr, 'c'},
        {nullptr, 0, nullptr, 0}
    };
    int opt;

    while ((opt = getopt_long(argc, argv, short_opts, long_opts, nullptr)) >= 0) {
		switch (opt) {
		    case 's':
			    sinfo_port = std::stoi(optarg);
			break;
		    case 'f':
			    fault_addr = optarg;
			    break;
		    case 'p':
			    fault_port = std::stoi(optarg);
			    break;
		    case 'd':
			    dbus_port = std::stoi(optarg);
			    break;
            case 'n':
                // FIXME: NotifyFault option for testing
                notify_fault = true;
                break;
            case 'c':
                node_config_file = optarg;
                break;
		    case 'h':
		    default:
                std::cerr << "Usage: " << argv[0] << " [options] [host]\n"
                          << "Options:\n"
                          << "  -s <port>\t\tPort for SchedInfoService (default: 50052)\n"
                          << "  -f <address>\t\tFaultService host address (default: localhost)\n"
                          << "  -p <port>\t\tPort for FaultService (default: 50053)\n"
                          << "  -d <port>\t\tPort for DBusServer (default: 7777)\n"
                          << "  -n\t\t\tEnable NotifyFault demo (default: false)\n"
                          << "  -c, --node-config <file>\tNode configuration YAML file\n"
                          << "  -h\t\t\tShow this help message\n";
                std::cerr << "Example: " << argv[0]
                          << " -s 50052 -f localhost -p 50053 -d 7777 --node-config examples/node_configurations.yaml\n";
                return false;
		}
	}

	return true;
}

int main(int argc, char **argv)
{
    int sinfo_port = 50052;
    std::string fault_addr = "localhost";
    int fault_port = 50053;
    int dbus_port = 7777;
    bool notify_fault = false; // Flag for NotifyFault method demo
    std::string node_config_file; // Node configuration file path

    if (!GetOptions(argc, argv, sinfo_port, fault_addr, fault_port,
                    dbus_port, notify_fault, node_config_file)) {
        exit(EXIT_FAILURE);
    }

    // Initialize the logger
    TLOG_SET_LOG_LEVEL(LogLevel::DEBUG);
    TLOG_SET_PRINT_FILENAME(false);
    TLOG_SET_FULL_TIMESTAMP(false);

    // Load node configuration if provided
    std::shared_ptr<NodeConfigManager> node_config_manager = std::make_shared<NodeConfigManager>();

    if (!node_config_file.empty()) {
        TLOG_INFO("Loading node configuration from: ", node_config_file);
        if (!node_config_manager->LoadFromFile(node_config_file)) {
            TLOG_ERROR("Failed to load node configuration, using default settings");
        }
    } else {
        TLOG_INFO("No node configuration file provided, using default node settings");
    }

    // Run the gRPC SchedInfoService server (with internal scheduling and node config)
    std::unique_ptr<SchedInfoServer> sinfo_server;
    if (!RunSchedInfoServer(sinfo_port, sinfo_server, node_config_manager)) {
        return EXIT_FAILURE;
    }

    // Initialize the gRPC FaultServiceClient
    if (!InitFaultClient(fault_addr, fault_port)) {
        return EXIT_FAILURE;
    }

    // Run the DBusServer
    if (!RunDBusServer(dbus_port, sinfo_server.get())) {
        return EXIT_FAILURE;
    }

    while (true) {
        std::this_thread::sleep_for(std::chrono::seconds(10));
        if (notify_fault) {
            if (NotifyFaultDemo()) {
                notify_fault = false; // Reset the flag after notification
            }
        }
    }

    return EXIT_SUCCESS;
}
