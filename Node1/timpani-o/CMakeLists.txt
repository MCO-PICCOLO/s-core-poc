# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.5)

# Read version from VERSION file
file(READ "${CMAKE_SOURCE_DIR}/VERSION" VERSION_CONTENT)
string(STRIP "${VERSION_CONTENT}" PROJECT_VERSION_STRING)

# Project name and version
project(timpani-o VERSION ${PROJECT_VERSION_STRING} DESCRIPTION "Timpani-O Orchestrator")

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)

# Set cmake module path
list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

# Add the binary dir to include dirs for gRPC/protobuf generated files
include_directories(${CMAKE_CURRENT_BINARY_DIR})

# 1. Integration of libtrpc
# First try: external/libtrpc (for container builds with submodule)
# Fallback: ../libtrpc (for local builds with sibling directory)
if(EXISTS "${CMAKE_SOURCE_DIR}/external/libtrpc/CMakeLists.txt")
    message(STATUS "Using libtrpc from external/libtrpc (submodule)")
    add_subdirectory("${CMAKE_SOURCE_DIR}/external/libtrpc" "${CMAKE_BINARY_DIR}/libtrpc" EXCLUDE_FROM_ALL)
    include_directories("${CMAKE_SOURCE_DIR}/external/libtrpc/src")
elseif(EXISTS "${CMAKE_SOURCE_DIR}/../libtrpc/CMakeLists.txt")
    message(STATUS "Using libtrpc from ../libtrpc (sibling directory)")
    add_subdirectory("${CMAKE_SOURCE_DIR}/../libtrpc" "${CMAKE_BINARY_DIR}/libtrpc" EXCLUDE_FROM_ALL)
    include_directories("${CMAKE_SOURCE_DIR}/../libtrpc/src")
else()
    message(FATAL_ERROR "libtrpc not found. Please either:\n"
        "  1. Initialize submodule: git submodule update --init --recursive\n"
        "  2. Place libtrpc in sibling directory: ../libtrpc")
endif()

# 2. Add local project components
add_subdirectory(proto)

# Add source directory
add_subdirectory(src)

# Option to build tests
option(BUILD_TESTS "Build unit tests" OFF)

# Add tests directory if building tests
if(BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

# Packaging configuration
set(CPACK_PACKAGE_NAME "timpani-o")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_CONTACT "Timpani Developers")
include(CPack)
