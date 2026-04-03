## SPDX header
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.5)

# project name and version
project(
  libtrpc
  VERSION 2.0.0
  DESCRIPTION "Timpani RPC library")

# check if pkg-config is available
find_package(PkgConfig)

if(PKG_CONFIG_FOUND)
  pkg_check_modules(LIBSYSTEMD libsystemd)
endif()

if (NOT LIBSYSTEMD_FOUND)
  # check if libsystemd configuration is defined manually
  if(DEFINED LIBSYSTEMD_INCLUDE_DIRS AND
     DEFINED LIBSYSTEMD_LIBRARIES AND
     DEFINED LIBSYSTEMD_VERSION)
    set(LIBSYSTEMD_FOUND TRUE)
    message(STATUS "Manually set libsystemd package configuration:")
    message(STATUS "  Include: ${LIBSYSTEMD_INCLUDE_DIRS}")
    message(STATUS "  Libraries: ${LIBSYSTEMD_LIBRARIES}")
    message(STATUS "  Version: ${LIBSYSTEMD_VERSION}")
  else()
    message(FATAL_ERROR "Run cmake as follows for libsystemd:\n"
            "cmake -DLIBSYSTEMD_INCLUDE_DIRS=/path/to/include "
            "-DLIBSYSTEMD_LIBRARIES=/path/to/lib/libsystemd.so "
            "-DLIBSYSTEMD_VERSION=250")
  endif()
else()
  # get the libsystemd version
  execute_process(
    COMMAND ${PKG_CONFIG_EXECUTABLE} --modversion libsystemd
    OUTPUT_VARIABLE LIBSYSTEMD_VERSION
    OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()

# set DEBUG=1 if CMAKE_BUILD_TYPE=Debug
add_compile_definitions("DEBUG=$<CONFIG:Debug>")

# libtrpc library
add_library(trpc SHARED src/serialize.c src/peer_dbus.c)

# set libtrpc so version
set_target_properties(trpc PROPERTIES VERSION ${PROJECT_VERSION}
                                      SOVERSION ${PROJECT_VERSION_MAJOR})

set_target_properties(trpc PROPERTIES PUBLIC_HEADER src/libtrpc.h)

# set libtrpc compile definitions
target_compile_definitions(trpc
                           PRIVATE "LIBSYSTEMD_VERSION=${LIBSYSTEMD_VERSION}")

# libsystemd required for library linking
target_link_libraries(trpc PUBLIC ${LIBSYSTEMD_LIBRARIES})

# library install config
install(
  TARGETS trpc
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

# trpc_server test program
add_executable(trpc_server test/trpc_server.c)
target_link_libraries(trpc_server trpc ${LIBSYSTEMD_LIBRARIES})
target_include_directories(
  trpc_server
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src
  PUBLIC ${LIBSYSTEMD_INCLUDE_DIRS})
target_compile_options(trpc_server PUBLIC ${LIBSYSTEMD_CFLAGS_OTHER})

# trpc_client test program
add_executable(trpc_client test/trpc_client.c)
target_link_libraries(trpc_client trpc ${LIBSYSTEMD_LIBRARIES})
target_include_directories(
  trpc_client
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src
  PUBLIC ${LIBSYSTEMD_INCLUDE_DIRS})
target_compile_options(trpc_client PUBLIC ${LIBSYSTEMD_CFLAGS_OTHER})

# Packaging configuration
set(CPACK_PACKAGE_NAME "libtrpc")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_CONTACT "Timpani Developers")
include(CPack)
