# Timpani-O executable
add_executable(timpani-o
  main.cpp
  schedinfo_service.cpp
  fault_client.cpp
  dbus_server.cpp
  node_config.cpp
  global_scheduler.cpp
  scheduler_utils.cpp
  hyperperiod_manager.cpp
)

target_include_directories(timpani-o
  PRIVATE
    ${CMAKE_SOURCE_DIR}/external/libtrpc/src
)

target_link_libraries(timpani-o
  PRIVATE
    schedinfo_proto
  PUBLIC
    trpc
)

install(TARGETS timpani-o
  RUNTIME DESTINATION bin
)

set_target_properties(timpani-o
  PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}"
)
