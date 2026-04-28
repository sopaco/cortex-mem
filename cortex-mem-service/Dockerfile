FROM ubuntu:24.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY target/release/cortex-mem-service /cortex-mem-service

ENV CORTEX_DATA_DIR=/mnt/sata-trace/cortex-mem/data
ENV CORTEX_TENANT_ID=tenant_claw

ENTRYPOINT ["/cortex-mem-service"]
