## SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
## SPDX-License-Identifier: MIT

set(LIBBPF_SOURCE_DIR ${CMAKE_SOURCE_DIR}/libbpf/src)
set(LIBBPF_BINARY_DIR ${CMAKE_BINARY_DIR}/libbpf)

# libbpf library as an external project
ExternalProject_Add(libbpf
	PREFIX libbpf
	SOURCE_DIR ${LIBBPF_SOURCE_DIR}
	CONFIGURE_COMMAND ""
	BUILD_COMMAND make
		BUILD_STATIC_ONLY=1
		OBJDIR=${LIBBPF_BINARY_DIR}/obj
		DESTDIR=${LIBBPF_BINARY_DIR}
		INCLUDEDIR=
		LIBDIR=
		UAPIDIR=
		CC=${CMAKE_C_COMPILER}
		LD=${CMAKE_LINKER}
		install install_uapi_headers
	BUILD_IN_SOURCE TRUE
	INSTALL_COMMAND ""
	STEP_TARGETS build
)

# import libbpf as bpf target
add_library(bpf STATIC IMPORTED)
set_target_properties(bpf PROPERTIES
	IMPORTED_LOCATION ${LIBBPF_BINARY_DIR}/libbpf.a
	INTERFACE_INCLUDE_DIRECTORIES ${LIBBPF_BINARY_DIR}
)
target_link_libraries(bpf INTERFACE elf z)
# let 'make clean' clean up the whole libbpf output
set_property(DIRECTORY
	APPEND
	PROPERTY ADDITIONAL_CLEAN_FILES ${LIBBPF_BINARY_DIR}
)
install(DIRECTORY ${LIBBPF_BINARY_DIR}/ TYPE LIB
	PATTERN "bpf" EXCLUDE
	PATTERN "linux" EXCLUDE
	PATTERN "obj" EXCLUDE
	PATTERN "pkgconfig" EXCLUDE
	PATTERN "src" EXCLUDE
	PATTERN "tmp" EXCLUDE
	PATTERN "libbpf.a" EXCLUDE
)
