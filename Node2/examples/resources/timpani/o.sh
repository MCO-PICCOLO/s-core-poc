#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

podman run -it -d --name timpani-o -p 50052:50052 -p 7777:7777 -v ./node_configurations.yaml:/timpani-o/examples/node_configurations.yaml sdv.lge.com/timpani/timpani-o:v0.1.0 -s 50052 -d 7777 -c /timpani-o/examples/node_configurations.yaml
