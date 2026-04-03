# Safe Exit Assist (SEA) — Container Image
#
# Build:
#   cargo build --release
#   docker build -t sdv.lge.com/demo/sea_app:1.0 .
#
# Run (standalone timer mode, no Timpani-N needed):
#   docker run --rm --network host sdv.lge.com/demo/sea_app:1.0 --timer --period-ms 100
#
# Run (signal mode with Timpani-N — requires host PID namespace):
#   docker run --rm --network host --pid host sdv.lge.com/demo/sea_app:1.0

FROM debian:bookworm-slim

WORKDIR /app

COPY target/debug/sea_app /app/sea_app

ENV RUST_LOG=warn

ENTRYPOINT ["/app/sea_app"]
