# Safe Exit Assist (SEA) — Container Image
#
# Build:
#   cargo build --release
#   docker build -t sdv.lge.com/demo/sea_app2:1.0 .
#
# Run (standalone timer mode, no Timpani-N needed):
#   docker run --rm --network host sdv.lge.com/demo/sea_app2:1.0 --timer --period-ms 100
#
# Run (signal mode with Timpani-N — requires host PID namespace):
#   docker run --rm --network host --pid host sdv.lge.com/demo/sea_app2:1.0

FROM ubuntu:24.04

WORKDIR /app

COPY target/release/sea_app2 /app/sea_app2

ENV RUST_LOG=warn

ENTRYPOINT ["/app/sea_app2"]
