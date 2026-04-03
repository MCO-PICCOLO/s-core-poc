# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.5)

# Project name and version
project(timpani-o VERSION 0.1.0 DESCRIPTION "Timpani-O")

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)

# Set cmake module path
list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

# Add the binary dir to include dirs for gRPC/protobuf generated files
include_directories(${CMAKE_CURRENT_BINARY_DIR})

# 1. Integration of libtrpc (Sibling folder in root)
# This points to the libtrpc folder located at the same level as timpani-o
if(EXISTS "${CMAKE_SOURCE_DIR}/../libtrpc")
    add_subdirectory("${CMAKE_SOURCE_DIR}/../libtrpc" "${CMAKE_BINARY_DIR}/libtrpc" EXCLUDE_FROM_ALL)

    # Ensure headers for libtrpc are available to timpani-o
    include_directories("${CMAKE_SOURCE_DIR}/../libtrpc/src")
else()
    message(WARNING "libtrpc sibling directory not found at ${CMAKE_SOURCE_DIR}/../libtrpc")
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
