cmake_minimum_required(VERSION 3.6)
## SPDX header
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.6)

# project name and version
project(sample_apps VERSION 0.1.0 DESCRIPTION "Timpani Sample Apps")

# library install config
install(TARGETS
	ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
	LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
	PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# sample_apps program
add_executable(sample_apps src/sample_apps.c)
target_include_directories(sample_apps
			PUBLIC ${CMAKE_SOURCE_DIR}/src
)
target_compile_options(sample_apps
		PUBLIC ${LIBSYSTEMD_CFLAGS_OTHER}
)
target_link_libraries(sample_apps PUBLIC m)
