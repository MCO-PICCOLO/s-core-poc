## SPDX header
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.5)

# project name and version
project(timpani-n VERSION 2.0.0 DESCRIPTION "Timpani-n")

# Set cmake module path
list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}")

# BPF tracing build options
option(CONFIG_TRACE_BPF "Enable eBPF support" ON)
option(CONFIG_TRACE_BPF_EVENT "Enable eBPF-based event tracing" OFF)

if(CONFIG_TRACE_BPF)
    message("CONFIG_TRACE_BPF is ON")

    include(ExternalProject)

    # Path to the shared libbpf in the TIMPANI root
    set(SHARED_LIBBPF_PATH "${CMAKE_SOURCE_DIR}/../libbpf")

    # Manually define libbpf project to point to the shared root
    ExternalProject_Add(libbpf
        SOURCE_DIR "${SHARED_LIBBPF_PATH}"
        CONFIGURE_COMMAND ""
        BUILD_COMMAND make -C <SOURCE_DIR>/src BUILD_STATIC_ONLY=1 DESTDIR=<BINARY_DIR> OBJDIR=<BINARY_DIR>/obj install
        INSTALL_COMMAND ""
        BINARY_DIR "${CMAKE_BINARY_DIR}/libbpf"
    )

    # Load bpf.cmake - we must help clang find headers in the shared root
    include(bpf)

    # We pass the shared path to Clang so #include <bpf/bpf_helpers.h> works
    # We use -I.../src because the 'bpf' folder is inside 'src'
    set(BPF_EXTRA_CFLAGS "-I${SHARED_LIBBPF_PATH}/src")

    add_compile_definitions(CONFIG_TRACE_BPF=1)

    # IMPORTANT: We need to make sure the ADD_BPF macro uses our new path
    # If sigwait.bpf.c fails to find headers, we might need to edit bpf.cmake
    include_directories(${CMAKE_BINARY_DIR}/libbpf/usr/include)
    ADD_BPF(src/trace_bpf.c src/sigwait.bpf.c sigwait.bpf.o sigwait.skel.h)

    if(CONFIG_TRACE_BPF_EVENT)
        message("CONFIG_TRACE_BPF_EVENT is ON")
        add_compile_definitions(CONFIG_TRACE_BPF_EVENT=1)
        # Add this right before ADD_BPF in CMakeLists.txt
        ADD_BPF(src/trace_bpf.c src/schedstat.bpf.c schedstat.bpf.o schedstat.skel.h)
    endif()
endif()

# timpani-n sources
set(TIMPANI_N_SOURCES
    src/main.c
	src/core.c
	src/hyperperiod.c
	src/task.c
    src/config.c
	src/trpc.c
	src/signal.c
	src/cleanup.c
    src/sched.c
	src/globals.c
	src/apex_monitor.c
)

if(CONFIG_TRACE_BPF)
    set(TIMPANI_N_SOURCES ${TIMPANI_N_SOURCES} src/trace_bpf.c)
endif()

add_executable(timpani-n ${TIMPANI_N_SOURCES})

# BPF dependencies
if(CONFIG_TRACE_BPF)
    add_dependencies(timpani-n libbpf)
    target_include_directories(timpani-n PRIVATE
        ${CMAKE_BINARY_DIR}
        ${CMAKE_BINARY_DIR}/libbpf/usr/include
    )
    target_link_libraries(timpani-n
        ${CMAKE_BINARY_DIR}/libbpf/obj/libbpf.a
        pthread elf z
    )
endif()

# Optimization
target_compile_options(timpani-n PRIVATE -O3 -flto -funroll-loops -ffast-math -fomit-frame-pointer)
set_target_properties(timpani-n PROPERTIES LINK_FLAGS "-flto -O3")

# Handle libtrpc (Sibling folder)
if(EXISTS "${CMAKE_SOURCE_DIR}/../libtrpc")
    add_subdirectory("${CMAKE_SOURCE_DIR}/../libtrpc" "${CMAKE_BINARY_DIR}/libtrpc" EXCLUDE_FROM_ALL)
endif()

target_link_libraries(timpani-n trpc systemd rt)
target_include_directories(timpani-n
    PUBLIC ${CMAKE_SOURCE_DIR}/src
    PUBLIC ${CMAKE_SOURCE_DIR}/../libtrpc/src
)

install(TARGETS timpani-n DESTINATION bin)
add_executable(exprocs EXCLUDE_FROM_ALL test/exprocs.c)

include(CPack)
