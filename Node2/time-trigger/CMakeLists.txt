cmake_minimum_required(VERSION 3.5)

# Read version from VERSION file
file(READ "${CMAKE_SOURCE_DIR}/VERSION" VERSION_CONTENT)
string(STRIP "${VERSION_CONTENT}" PROJECT_VERSION_STRING)

# project name and version
project(timpani-n VERSION ${PROJECT_VERSION_STRING} DESCRIPTION "Timpani Time Trigger")

# Set cmake module path
list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}")

# BPF tracing build options
option(CONFIG_TRACE_BPF "Enable eBPF support" ON)
option(CONFIG_TRACE_BPF_EVENT "Enable eBPF-based event tracing" OFF)

if(CONFIG_TRACE_BPF)
    message("CONFIG_TRACE_BPF is ON")

    # for ExternalProject_Add
    include(ExternalProject)

    # load libbpf.cmake
    include(libbpf)

    # Load bpf.cmake
    include(bpf)

    add_compile_definitions(CONFIG_TRACE_BPF=1)
    ADD_BPF(src/trace_bpf.c src/sigwait.bpf.c sigwait.bpf.o sigwait.skel.h)

if(CONFIG_TRACE_BPF_EVENT)
    message("CONFIG_TRACE_BPF_EVENT is ON")
    add_compile_definitions(CONFIG_TRACE_BPF_EVENT=1)
    ADD_BPF(src/trace_bpf.c src/schedstat.bpf.c schedstat.bpf.o schedstat.skel.h)
endif()

endif()

# timetrigger sources - integrated scheduling and tracing
set(TIMETRIGGER_SOURCES
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

# Add BPF tracing sources based on configuration
if(CONFIG_TRACE_BPF)
    set(TIMETRIGGER_SOURCES ${TIMETRIGGER_SOURCES} src/trace_bpf.c)
endif()

add_executable(timpani-n ${TIMETRIGGER_SOURCES})

# BPF dependencies for timpani-n
if(CONFIG_TRACE_BPF)
    add_dependencies(timpani-n libbpf)
    target_include_directories(timpani-n PRIVATE ${CMAKE_BINARY_DIR})
    target_link_libraries(timpani-n bpf pthread)
endif()

# Compile options for perfomance optimization
target_compile_options(timpani-n PRIVATE
    -O3
    -flto
    -funroll-loops
    -finline-functions
    -ffast-math
    -fomit-frame-pointer
)

# Link options for optimization
set_target_properties(timpani-n PROPERTIES
    LINK_FLAGS "-flto -O3"
)

target_link_libraries(timpani-n
    trpc
    systemd
    rt
)
target_include_directories(timpani-n
    PUBLIC ${CMAKE_SOURCE_DIR}/src
    PUBLIC ${CMAKE_SOURCE_DIR}/libtrpc/src
)
target_compile_options(timpani-n
    PUBLIC ${LIBSYSTEMD_CFLAGS_OTHER}
)
install(TARGETS timpani-n DESTINATION ${CMAKE_INSTALL_BINDIR})

# Install check-requirements script
install(PROGRAMS ${CMAKE_SOURCE_DIR}/scripts/check-requirements.sh
        DESTINATION share/timpani-n)

# Install systemd service file
install(FILES ${CMAKE_SOURCE_DIR}/timpani-n.service
        DESTINATION lib/systemd/system)

# Install documentation
install(FILES ${CMAKE_SOURCE_DIR}/README.md
              ${CMAKE_SOURCE_DIR}/INSTALL.md
        DESTINATION share/doc/timpani-n)
install(DIRECTORY ${CMAKE_SOURCE_DIR}/docs/
        DESTINATION share/doc/timpani-n/docs)

# exprocs test program
add_executable(exprocs EXCLUDE_FROM_ALL test/exprocs.c)

# libtrpc library
add_subdirectory(libtrpc)

# ============================================
# Packaging configuration (CPack)
# ============================================

# Map architecture names for Debian packages
# CMAKE_SYSTEM_PROCESSOR returns: x86_64, aarch64, armv7l, etc.
# Debian expects: amd64, arm64, armhf, etc.
set(DEB_ARCH ${CMAKE_SYSTEM_PROCESSOR})
if(CMAKE_SYSTEM_PROCESSOR STREQUAL "x86_64")
    set(DEB_ARCH "amd64")
elseif(CMAKE_SYSTEM_PROCESSOR STREQUAL "aarch64")
    set(DEB_ARCH "arm64")
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "^arm")
    set(DEB_ARCH "armhf")
endif()

set(CPACK_PACKAGE_NAME "timpani-n")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Time-triggered task scheduler with eBPF monitoring")
set(CPACK_PACKAGE_DESCRIPTION "Timpani-N is a time-triggered task scheduler and monitor for real-time Linux systems. It uses eBPF-based instrumentation to detect deadline misses and provides fine-grained control over task scheduling.")
set(CPACK_PACKAGE_VENDOR "LG Electronics")
set(CPACK_PACKAGE_CONTACT "timpani-dev@lge.com")
if(EXISTS "${CMAKE_SOURCE_DIR}/LICENSE")
    set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_SOURCE_DIR}/LICENSE")
endif()
set(CPACK_RESOURCE_FILE_README "${CMAKE_SOURCE_DIR}/README.md")

# DEB package settings
set(CPACK_DEBIAN_PACKAGE_DEPENDS "libelf1, zlib1g, libsystemd0")
set(CPACK_DEBIAN_PACKAGE_SECTION "utils")
set(CPACK_DEBIAN_PACKAGE_PRIORITY "optional")
set(CPACK_DEBIAN_PACKAGE_HOMEPAGE "http://mod.lge.com/hub/timpani/time-trigger")
set(CPACK_DEBIAN_PACKAGE_CONTROL_EXTRA "${CMAKE_SOURCE_DIR}/scripts/postinst")
set(CPACK_DEBIAN_FILE_NAME "timpani-n_${PROJECT_VERSION}_${DEB_ARCH}.deb")

# RPM package settings
set(CPACK_RPM_PACKAGE_REQUIRES "elfutils-libelf, zlib, systemd-libs")
set(CPACK_RPM_PACKAGE_LICENSE "MIT")
set(CPACK_RPM_PACKAGE_GROUP "System Environment/Daemons")
set(CPACK_RPM_PACKAGE_URL "http://mod.lge.com/hub/timpani/time-trigger")
set(CPACK_RPM_POST_INSTALL_SCRIPT_FILE "${CMAKE_SOURCE_DIR}/scripts/postinst")
set(CPACK_RPM_FILE_NAME "timpani-n-${PROJECT_VERSION}.${CMAKE_SYSTEM_PROCESSOR}.rpm")

include(CPack)
