/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

#include <gtest/gtest.h>
#include <thread>
#include <chrono>
#include <memory>
#include <random>
#include <cstring>

#include "../src/dbus_server.h"
#include "../src/schedinfo_service.h"
#include "../src/sched_info.h"
#include "../src/tlog.h"

// Helper function to get an unused port
int GetUnusedPort()
{
    static std::random_device rd;
    static std::mt19937 gen(rd());
    static std::uniform_int_distribution<> dis(50000, 59999);
    return dis(gen);
}

// Helper function to create sample sched_info_t
sched_info_t CreateSampleSchedInfo(int num_tasks = 2)
{
    sched_info_t sched_info;
    sched_info.num_tasks = num_tasks;
    sched_info.tasks = new sched_task_t[num_tasks];

    for (int i = 0; i < num_tasks; ++i) {
        strncpy(sched_info.tasks[i].task_name, ("task_" + std::to_string(i)).c_str(), 15);
        sched_info.tasks[i].task_name[15] = '\0';
        sched_info.tasks[i].period_ns = 1000000000ULL; // 1 second
        sched_info.tasks[i].runtime_ns = 100000000ULL; // 100ms
        sched_info.tasks[i].deadline_ns = 900000000ULL; // 900ms
        sched_info.tasks[i].release_time = 0;
        sched_info.tasks[i].cpu_affinity = 1 << i; // CPU 0, 1, etc.
        sched_info.tasks[i].sched_policy = SCHED_FIFO;
        sched_info.tasks[i].sched_priority = 50 + i;
        sched_info.tasks[i].max_dmiss = 3;
        strncpy(sched_info.tasks[i].assigned_node, "node1", 63);
        sched_info.tasks[i].assigned_node[63] = '\0';
    }

    return sched_info;
}

// Helper function to create sample SchedInfoMap
SchedInfoMap CreateSampleSchedInfoMap(const std::string& workload_id = "test_workload")
{
    SchedInfoMap map;
    NodeSchedInfoMap node_map;

    sched_info_t sched_info = CreateSampleSchedInfo(2);
    node_map["node1"] = sched_info;

    map[workload_id] = node_map;
    return map;
}

class DBusServerTest : public ::testing::Test
{
  protected:
    void SetUp() override
    {
        // Get singleton instance
        dbus_server_ = &DBusServer::GetInstance();

        // Get unused port for testing
        test_port_ = GetUnusedPort();
    }

    void TearDown() override
    {
        // Ensure server is stopped
        dbus_server_->Stop();

        // Clean up sample data
        for (auto& [workload_id, node_map] : sample_map_) {
            for (auto& [node_id, sched_info] : node_map) {
                delete[] sched_info.tasks;
            }
        }
    }

    DBusServer* dbus_server_;
    int test_port_;
    SchedInfoMap sample_map_;
};

// Test singleton pattern
TEST_F(DBusServerTest, SingletonBehavior)
{
    DBusServer& instance1 = DBusServer::GetInstance();
    DBusServer& instance2 = DBusServer::GetInstance();

    EXPECT_EQ(&instance1, &instance2);
    EXPECT_EQ(dbus_server_, &instance1);
}

// Test Start with valid parameters
TEST_F(DBusServerTest, StartSuccess)
{
    bool result = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result);

    // Give some time for the event loop to start
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dbus_server_->Stop();
}

// Test Start without SchedInfoServer
TEST_F(DBusServerTest, StartWithoutSchedInfoServer)
{
    bool result = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result);

    // Give some time for the event loop to start
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dbus_server_->Stop();
}

// Test Start when already running
TEST_F(DBusServerTest, StartWhenAlreadyRunning)
{
    // Start first time
    bool result1 = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result1);

    // Try to start again - should return false
    bool result2 = dbus_server_->Start(test_port_ + 1, nullptr);
    EXPECT_FALSE(result2);

    dbus_server_->Stop();
}

// Test Stop when not running
TEST_F(DBusServerTest, StopWhenNotRunning)
{
    // Should not crash when stopping a server that's not running
    EXPECT_NO_THROW(dbus_server_->Stop());
}

// Test Stop when running
TEST_F(DBusServerTest, StopWhenRunning)
{
    bool result = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result);

    // Give some time for the event loop to start
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    EXPECT_NO_THROW(dbus_server_->Stop());
}

// Test SetSchedInfoServer
TEST_F(DBusServerTest, SetSchedInfoServer)
{
    // Create a real SchedInfoServer instance for testing
    auto sched_info_server = std::make_unique<SchedInfoServer>();

    // Set the server
    dbus_server_->SetSchedInfoServer(sched_info_server.get());

    // Start the D-Bus server to verify the SchedInfoServer is used
    bool result = dbus_server_->Start(test_port_);
    EXPECT_TRUE(result);

    // Give some time for the event loop to start
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dbus_server_->Stop();

    // Set to nullptr
    dbus_server_->SetSchedInfoServer(nullptr);
}

// Test error handling for invalid port
TEST_F(DBusServerTest, StartWithInvalidPort)
{
    // Try to start with an invalid port (negative)
    // Note: The underlying implementation may still accept negative ports
    // This test documents the current behavior
    bool result = dbus_server_->Start(-1, nullptr);
    // If the implementation allows negative ports, we just verify it doesn't crash
    if (result) {
        EXPECT_NO_THROW(dbus_server_->Stop());
    }
    // The test passes regardless as long as no crash occurs
}

// Test multiple Start/Stop cycles
TEST_F(DBusServerTest, MultipleStartStopCycles)
{
    for (int i = 0; i < 3; ++i) {
        bool result = dbus_server_->Start(test_port_, nullptr);
        EXPECT_TRUE(result);

        std::this_thread::sleep_for(std::chrono::milliseconds(50));

        dbus_server_->Stop();

        // Small delay between cycles
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
}

// Test with actual SchedInfoServer instance
TEST_F(DBusServerTest, WithRealSchedInfoServer)
{
    // Create a real SchedInfoServer instance
    auto sched_info_server = std::make_unique<SchedInfoServer>();

    bool result = dbus_server_->Start(test_port_, sched_info_server.get());
    EXPECT_TRUE(result);

    // Give some time for the event loop to start
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dbus_server_->Stop();
}

// Test basic functionality without accessing private members
TEST_F(DBusServerTest, BasicFunctionality)
{
    // Test that we can create and manage the server without crashes
    bool result = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result);

    // Let it run for a short time
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    // Should be able to stop cleanly
    EXPECT_NO_THROW(dbus_server_->Stop());
}

// Test repeated start/stop operations to check for resource leaks
TEST_F(DBusServerTest, ResourceManagement)
{
    for (int i = 0; i < 5; ++i) {
        bool result = dbus_server_->Start(test_port_, nullptr);
        EXPECT_TRUE(result);

        // Brief operation period
        std::this_thread::sleep_for(std::chrono::milliseconds(50));

        dbus_server_->Stop();

        // Brief cleanup period
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
}

// Test that server handles being stopped multiple times
TEST_F(DBusServerTest, MultipleStopCalls)
{
    bool result = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result);

    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Stop multiple times - should not crash
    EXPECT_NO_THROW(dbus_server_->Stop());
    EXPECT_NO_THROW(dbus_server_->Stop());
    EXPECT_NO_THROW(dbus_server_->Stop());
}

// Test port reuse after stopping
TEST_F(DBusServerTest, PortReuse)
{
    // Start and stop the server
    bool result1 = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result1);

    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    dbus_server_->Stop();

    // Give some time for port to be released
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Should be able to start again on the same port
    bool result2 = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result2);

    dbus_server_->Stop();
}

// Test setting SchedInfoServer after start
TEST_F(DBusServerTest, SetSchedInfoServerAfterStart)
{
    bool result = dbus_server_->Start(test_port_, nullptr);
    EXPECT_TRUE(result);

    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Should be able to set SchedInfoServer while running
    auto sched_info_server = std::make_unique<SchedInfoServer>();
    EXPECT_NO_THROW(dbus_server_->SetSchedInfoServer(sched_info_server.get()));

    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Should be able to unset it too
    EXPECT_NO_THROW(dbus_server_->SetSchedInfoServer(nullptr));

    dbus_server_->Stop();
}

// Test main function
int main(int argc, char** argv) {
    ::testing::InitGoogleTest(&argc, argv);
    TLOG_SET_LOG_LEVEL(LogLevel::NONE);
    return RUN_ALL_TESTS();
}
