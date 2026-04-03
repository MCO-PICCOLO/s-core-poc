/*
 * Quick test client: sends a SchedInfo to timpani-o and prints the response.
 *
 * Usage:
 *   test_client [host:port] [cpu_affinity_hex] [task_name] [period_us] [runtime_us] [priority] [policy]
 *
 *   policy: 0=SCHED_OTHER  1=SCHED_FIFO  2=SCHED_RR
 *
 * Defaults match sea_app: task_name="sea_app", period=100000 (100ms), runtime=10000 (10ms)
 * For deadline-miss stress test (SCHED_OTHER so stress workers can starve it):
 *   test_client localhost:50052 0x100 sea_app 10000 1000 0 0
 */
#include <iostream>
#include <memory>
#include <string>
#include <grpcpp/grpcpp.h>
#include "schedinfo.grpc.pb.h"

using grpc::Channel;
using grpc::ClientContext;
using grpc::Status;
using schedinfo::v1::SchedInfo;
using schedinfo::v1::SchedInfoService;
using schedinfo::v1::TaskInfo;
using schedinfo::v1::Response;
using schedinfo::v1::SchedPolicy;

int main(int argc, char* argv[])
{
    std::string target       = "localhost:50052";
    uint64_t    cpu_affinity = 0xF;
    std::string task_name    = "sea_app";
    uint32_t    period_us    = 100000;   // 100ms
    uint32_t    runtime_us   = 10000;    // 10ms
    int         priority     = 50;
    int         policy_int   = 1;        // 1=SCHED_FIFO

    if (argc > 1) target       = argv[1];
    if (argc > 2) cpu_affinity = std::stoull(argv[2], nullptr, 0);
    if (argc > 3) task_name    = argv[3];
    if (argc > 4) period_us    = std::stoul(argv[4]);
    if (argc > 5) runtime_us   = std::stoul(argv[5]);
    if (argc > 6) priority     = std::stoi(argv[6]);
    if (argc > 7) policy_int   = std::stoi(argv[7]);

    SchedPolicy policy;
    switch (policy_int) {
        case 0:  policy = SchedPolicy::NORMAL; break;
        case 2:  policy = SchedPolicy::RR;     break;
        default: policy = SchedPolicy::FIFO;   break;
    }

    auto channel = grpc::CreateChannel(target, grpc::InsecureChannelCredentials());
    auto stub = SchedInfoService::NewStub(channel);

    SchedInfo request;
    request.set_workload_id("sea_workload");

    TaskInfo* t = request.add_tasks();
    t->set_name(task_name);
    t->set_priority(priority);
    t->set_policy(policy);
    t->set_cpu_affinity(cpu_affinity);
    t->set_period(period_us);
    t->set_runtime(runtime_us);
    t->set_deadline(period_us);
    t->set_release_time(0);
    t->set_max_dmiss(3);
    t->set_node_id("node01");

    std::cout << "Sending task=" << task_name
              << " cpu_affinity=0x" << std::hex << cpu_affinity
              << " period=" << std::dec << period_us << "us"
              << " runtime=" << runtime_us << "us"
              << " priority=" << priority
              << " policy=" << policy_int << "\n";

    Response reply;
    ClientContext ctx;
    Status status = stub->AddSchedInfo(&ctx, request, &reply);

    if (!status.ok()) {
        std::cerr << "RPC failed: " << status.error_message() << "\n";
        return 1;
    }

    std::cout << "Response status: " << reply.status() << " (0 = success)\n";
    return 0;
}

