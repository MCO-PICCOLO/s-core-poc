#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

#BODY=$(< ./resources/helloworld.yaml)
BODY=$(< ./resources/helloworld_no_condition.yaml)

#URL="10.0.0.30:8080/api/v1/yaml"

curl -X POST 'http://127.0.0.1:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"
