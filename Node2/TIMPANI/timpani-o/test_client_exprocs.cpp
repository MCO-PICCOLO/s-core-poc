/*
 * Test client for exprocs — sends a SchedInfo to timpani-o.
 *
 * Usage:
 *   test_client_exprocs [host:port] [cpu_affinity_hex]
 *
 * Defaults: task_name="task_cpumask", period=10ms, runtime=2ms, CPU affinity=0xF
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

    if (argc > 1) target       = argv[1];
    if (argc > 2) cpu_affinity = std::stoull(argv[2], nullptr, 0);

    auto channel = grpc::CreateChannel(target, grpc::InsecureChannelCredentials());
    auto stub = SchedInfoService::NewStub(channel);

    SchedInfo request;
    request.set_workload_id("test_affinity");

    TaskInfo* t = request.add_tasks();
    t->set_name("task_cpumask");
    t->set_priority(50);
    t->set_policy(SchedPolicy::FIFO);
    t->set_cpu_affinity(cpu_affinity);
    t->set_period(10000);    // 10ms
    t->set_runtime(2000);    // 2ms
    t->set_deadline(10000);  // 10ms
    t->set_release_time(0);
    t->set_max_dmiss(3);
    t->set_node_id("node01");

    std::cout << "Sending task=task_cpumask cpu_affinity=0x" << std::hex << cpu_affinity
              << " period=10000us runtime=2000us\n";

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
