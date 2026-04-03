<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# Pullpiri REST API Testing Guide

This guide describes how to test Pullpiri REST API endpoints using `curl`.  
**Always run the full validation sequence before testing. Never cancel builds or lint checks.**

---

## 1. Environment Setup

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

---

## 2. Build and Validate (Mandatory)

```bash
scripts/fmt_check.sh
scripts/clippy_check.sh
make build
```

---

## 3. Start the Settings Service

```bash
cd src/server/settingsservice
cargo run --bin settingsservice

```

---

## 4. Test REST API Endpoints with curl

> **Note:**  
> For pod or container data to appear in API responses, there **must be at least one running container on the specified node**.  
> If no containers are running, the response will be empty.

### Node Management

```bash
curl http://localhost:8080/api/v1/nodes
curl http://localhost:8080/api/v1/nodes/{node_name}
```

### Pod Metrics (Enhanced)

```bash
curl http://localhost:8080/api/v1/nodes/{node_name}/pods/metrics
curl "http://localhost:8080/api/v1/nodes/{node_name}/pods/metrics?page=0&page_size=2"
```

### Containers by Node

```bash
curl http://localhost:8080/api/v1/nodes/{node_name}/containers
curl "http://localhost:8080/api/v1/nodes/{node_name}/containers?page=0&page_size=2"
```

### Container Management

```bash
curl http://localhost:8080/api/v1/containers
curl http://localhost:8080/api/v1/containers/{container_id}
curl -X POST http://localhost:8080/api/v1/containers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-vehicle-diagnostics",
    "image": "docker.io/library/alpine:3.21.3",
    "node_name": "{node_name}",
    "description": "Test vehicle diagnostic service",
    "labels": {
      "service": "diagnostics",
      "version": "1.0.0",
      "environment": "test"
    }
  }'
```

### Metrics Endpoints

```bash
curl http://localhost:8080/api/v1/metrics/containers
curl http://localhost:8080/api/v1/metrics/nodes
```

---

## 5. Pretty Print JSON Output

If you have `jq` installed:

```bash
curl -s http://localhost:8080/api/v1/nodes/{node_name}/pods/metrics | jq .
```

---

## 6. Error Case Testing

```bash
curl -v http://localhost:8080/api/v1/nodes/nonexistent/pods/metrics
curl -v "http://localhost:8080/api/v1/nodes/{node_name}/pods/metrics?page=-1&page_size=0"
```

---

## 7. Troubleshooting

- Check service logs:  
  `journalctl -u settingsservice -f`  
  or  
  `tail -f /tmp/settingsservice.log`
- Check etcd health:  
  `etcdctl --endpoints=http://localhost:2379 endpoint health`
- Check if service is running:  
  `netstat -tlnp | grep 8080`

---

**Always run the full validation sequence before testing. Never cancel build or lint operations.**

Apache-2.0 (following Pullpiri framework licensing)