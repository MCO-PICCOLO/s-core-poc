cmake_minimum_required(VERSION 3.5)

# project name and version
project(libtrpc VERSION 2.0.0 DESCRIPTION "Timpani RPC library")

# if pkg-config is not available, run 'sudo apt install pkg-config'
find_package(PkgConfig REQUIRED)

# pkg-config --cflags --libs libsystemd
# if libsystemd-dev is not available, run 'sudo apt install libsystemd-dev'
pkg_check_modules(LIBSYSTEMD REQUIRED libsystemd)

add_compile_definitions("DEBUG=$<CONFIG:Debug>")

# libtrpc library
add_library(trpc SHARED
	src/serialize.c
	src/peer_dbus.c
)

# set libtrpc so version
set_target_properties(trpc PROPERTIES
		VERSION ${PROJECT_VERSION}
		SOVERSION ${PROJECT_VERSION_MAJOR}
)

set_target_properties(trpc PROPERTIES
		PUBLIC_HEADER src/libtrpc.h
)

# library install config
install(TARGETS trpc
	ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
	LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
	PUBLIC_HEADER DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# get the libsystemd version
if(LIBSYSTEMD_FOUND)
    execute_process(
        COMMAND ${PKG_CONFIG_EXECUTABLE} --modversion libsystemd
        OUTPUT_VARIABLE LIBSYSTEMD_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    message(STATUS "Found libsystemd version: ${LIBSYSTEMD_VERSION}")

    target_compile_definitions(trpc PRIVATE "LIBSYSTEMD_VERSION=${LIBSYSTEMD_VERSION}")
endif()

# trpc_server test program
add_executable(trpc_server test/trpc_server.c)
target_link_libraries(trpc_server
		trpc
		${LIBSYSTEMD_LIBRARIES}
)
target_include_directories(trpc_server
			PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src
			PUBLIC ${LIBSYSTEMD_INCLUDE_DIRS}
)
target_compile_options(trpc_server
		PUBLIC ${LIBSYSTEMD_CFLAGS_OTHER}
)

# trpc_client test program
add_executable(trpc_client test/trpc_client.c)
target_link_libraries(trpc_client
		trpc
		${LIBSYSTEMD_LIBRARIES}
)
target_include_directories(trpc_client
			PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src
			PUBLIC ${LIBSYSTEMD_INCLUDE_DIRS}
)
target_compile_options(trpc_client
		PUBLIC ${LIBSYSTEMD_CFLAGS_OTHER}
)
