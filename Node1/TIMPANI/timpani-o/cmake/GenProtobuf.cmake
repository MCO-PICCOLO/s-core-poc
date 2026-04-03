find_package(gRPC QUIET)
  if (NOT ${gRPC_FOUND} OR NOT TARGET gRPC::grpc_cpp_plugin)
    message(STATUS "gRPC not found via find_package or targets missing, trying pkg-config...")
    find_package(PkgConfig REQUIRED)
    pkg_search_module(GRPC REQUIRED IMPORTED_TARGET grpc)
    pkg_search_module(GRPCPP REQUIRED IMPORTED_TARGET grpc++)
    if (NOT TARGET gRPC::grpc)
      add_library(gRPC::grpc ALIAS PkgConfig::GRPC)
    endif()
    if (NOT TARGET gRPC::grpc++)
      add_library(gRPC::grpc++ ALIAS PkgConfig::GRPCPP)
    endif()
    if (NOT TARGET gRPC::grpc_cpp_plugin)
      add_executable(gRPC::grpc_cpp_plugin IMPORTED)
      find_program(GRPC_CPP_PLUGIN NAMES grpc_cpp_plugin REQUIRED)
      set_property(TARGET gRPC::grpc_cpp_plugin PROPERTY
        IMPORTED_LOCATION ${GRPC_CPP_PLUGIN}
      )
    endif()
  endif()
find_package(Protobuf REQUIRED)

include_directories(${Protobuf_INCLUDE_DIRS})

macro(GEN_PROTOBUF ProtoFile GenFiles ProtoLibs)
  protobuf_generate(
    OUT_VAR _proto_srcs
    LANGUAGE cpp
    PROTOS ${ProtoFile}
  )

  protobuf_generate(
    OUT_VAR _grpc_srcs
    LANGUAGE grpc
    GENERATE_EXTENSIONS .grpc.pb.h .grpc.pb.cc
    PLUGIN "protoc-gen-grpc=\$<TARGET_FILE:gRPC::grpc_cpp_plugin>"
    PROTOS ${ProtoFile}
  )

  set(${GenFiles} ${_proto_srcs} ${_grpc_srcs})
  set(${ProtoLibs} gRPC::grpc++ protobuf::libprotobuf)
endmacro()
