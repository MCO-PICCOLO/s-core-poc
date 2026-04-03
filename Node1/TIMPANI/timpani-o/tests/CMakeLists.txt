## SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
## SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.5)

# Find required packages
find_package(PkgConfig REQUIRED)
find_package(Protobuf REQUIRED)
find_package(gRPC REQUIRED)

# Find Google Test and Google Mock
find_package(GTest REQUIRED)

# Include directories
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../src)
include_directories(${CMAKE_CURRENT_BINARY_DIR}/../proto)
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../external/libtrpc/src)
include_directories(${GTEST_INCLUDE_DIRS})

# Create test executable
add_executable(test_schedinfo_service
    test_schedinfo_service.cpp
    ../src/schedinfo_service.cpp
    ../src/node_config.cpp
    ../src/global_scheduler.cpp
    ../src/scheduler_utils.cpp
    ../src/hyperperiod_manager.cpp
)

# Link libraries
target_link_libraries(test_schedinfo_service
    GTest::gtest
    GTest::gtest_main
    gRPC::grpc++
    protobuf::libprotobuf
    schedinfo_proto  # Proto library from proto directory
    pthread
)

# Add test
add_test(NAME SchedInfoServiceTest COMMAND test_schedinfo_service)

# Set test properties
set_tests_properties(SchedInfoServiceTest PROPERTIES
    TIMEOUT 30
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Create DBusServer test executable
add_executable(test_dbus_server
    test_dbus_server.cpp
    ../src/dbus_server.cpp
    ../src/fault_client.cpp
    ../src/schedinfo_service.cpp
    ../src/node_config.cpp
    ../src/global_scheduler.cpp
    ../src/scheduler_utils.cpp
    ../src/hyperperiod_manager.cpp
)

# Link libraries for DBusServer test
target_link_libraries(test_dbus_server
    GTest::gtest
    GTest::gtest_main
    gRPC::grpc++
    protobuf::libprotobuf
    schedinfo_proto  # Proto library from proto directory
    ${CMAKE_CURRENT_BINARY_DIR}/../external/libtrpc/libtrpc.so
    systemd
    pthread
)

# Add DBusServer test
add_test(NAME DBusServerTest COMMAND test_dbus_server)

# Set test properties for DBusServer test
set_tests_properties(DBusServerTest PROPERTIES
    TIMEOUT 60
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Create FaultServiceClient test executable
add_executable(test_fault_client
    test_fault_client.cpp
    ../src/fault_client.cpp
)

# Link libraries for FaultServiceClient test
target_link_libraries(test_fault_client
    GTest::gtest
    GTest::gtest_main
    gRPC::grpc++
    protobuf::libprotobuf
    schedinfo_proto  # Proto library from proto directory
    pthread
)

# Add FaultServiceClient test
add_test(NAME FaultServiceClientTest COMMAND test_fault_client)

# Set test properties for FaultServiceClient test
set_tests_properties(FaultServiceClientTest PROPERTIES
    TIMEOUT 30
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Create GlobalScheduler test executable
add_executable(test_global_scheduler
    test_global_scheduler.cpp
    ../src/global_scheduler.cpp
    ../src/node_config.cpp
    ../src/scheduler_utils.cpp
)

# Link libraries for GlobalScheduler test
target_link_libraries(test_global_scheduler
    GTest::gtest
    GTest::gtest_main
    pthread
)

# Add GlobalScheduler test
add_test(NAME GlobalSchedulerTest COMMAND test_global_scheduler)

# Set test properties for GlobalScheduler test
set_tests_properties(GlobalSchedulerTest PROPERTIES
    TIMEOUT 30
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Create NodeConfig test executable
add_executable(test_node_config
    test_node_config.cpp
    ../src/node_config.cpp
)

# Link libraries for NodeConfig test
target_link_libraries(test_node_config
    GTest::gtest
    GTest::gtest_main
    pthread
)

# Add NodeConfig test
add_test(NAME NodeConfigTest COMMAND test_node_config)

# Set test properties for NodeConfig test
set_tests_properties(NodeConfigTest PROPERTIES
    TIMEOUT 30
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)
