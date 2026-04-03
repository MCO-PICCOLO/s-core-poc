# Load a module to generate protobuf/gRPC source files
include(GenProtobuf)

# Generate protobuf source files for gRPC
GEN_PROTOBUF("schedinfo.proto" PROTOBUF_GEN_FILES PROTOBUF_LIBS)

# Add the generated source files to the library project
add_library(schedinfo_proto ${PROTOBUF_GEN_FILES})
target_link_libraries(schedinfo_proto PUBLIC ${PROTOBUF_LIBS})
