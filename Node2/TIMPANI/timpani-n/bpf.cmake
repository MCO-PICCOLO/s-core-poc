## SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
## SPDX-License-Identifier: MIT

set(BPF_ARCH ${CMAKE_SYSTEM_PROCESSOR})

if(NOT CMAKE_TOOLCHAIN_FILE)
    set(BPF_INCLUDES -I/usr/include/${BPF_ARCH}-linux-gnu)
else()
    set(BPF_INCLUDES -I/usr/${BPF_ARCH}-linux-gnu/include)
endif()

# --- FIX START ---
# Add the path to the libbpf headers we just built in the build directory
# This allows clang to find <bpf/bpf_helpers.h>
list(APPEND BPF_INCLUDES "-I${CMAKE_BINARY_DIR}/libbpf/usr/include")
# --- FIX END ---

set(VMLINUX_H_DIR ${CMAKE_SOURCE_DIR}/src/bpf/${BPF_ARCH})
set(BPF_INCLUDES ${BPF_INCLUDES} -I${VMLINUX_H_DIR})

if(LIBBPF_BINARY_DIR)
    set(BPF_INCLUDES ${BPF_INCLUDES} -I${LIBBPF_BINARY_DIR})
endif()

# ADD_BPF macro
macro(ADD_BPF Loader Input Output OutputSkel)

set(LOADER_SRC "${CMAKE_SOURCE_DIR}/${Loader}")
set(BPF_SRC "${CMAKE_SOURCE_DIR}/${Input}")
set(BPF_OBJ "${Output}")
set(SKEL_HDR "${OutputSkel}")

add_custom_command(OUTPUT ${BPF_OBJ}
        # We ensure BPF_INCLUDES is passed to clang
        COMMAND clang -target bpf -g -O2 -c ${BPF_SRC} -o ${BPF_OBJ} ${BPF_INCLUDES}
        VERBATIM
        # Added libbpf as a dependency so headers are built before clang runs
        DEPENDS ${BPF_SRC} ${VMLINUX_H_DIR}/vmlinux.h libbpf
)

add_custom_command(OUTPUT ${SKEL_HDR}
        # Use the versioned bpftool binary directly; the system wrapper at
        # /usr/sbin/bpftool emits a "not found for kernel X" error and exits
        # non-zero when the kernel-tools package for the running kernel is absent.
        COMMAND /usr/lib/linux-tools/6.8.0-106-generic/bpftool gen skeleton ${BPF_OBJ} > ${SKEL_HDR}
        VERBATIM
        DEPENDS ${BPF_OBJ}
)

get_source_file_property(LOADER_DEPENDS ${LOADER_SRC} OBJECT_DEPENDS)
if(${LOADER_DEPENDS} STREQUAL "NOTFOUND")
    unset(LOADER_DEPENDS)
endif()
set(LOADER_DEPENDS ${LOADER_DEPENDS} ${SKEL_HDR})

set_source_files_properties(${LOADER_SRC}
            PROPERTIES OBJECT_DEPENDS "${LOADER_DEPENDS}")

endmacro()