#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

#BODY=$(< ./resources/helloworld.yaml)
#BODY=$(< ./resources/timpani-test.yaml)
#BODY=$(< ./resources/timpani-test-1proc.yaml)
#BODY=$(< ./resources/timpani-test-1proc-nomiss.yaml)
BODY=$(< ./resources/safe-exit-assist.yaml)
#URL="10.0.0.30:8080/api/v1/yaml"

curl -X POST 'http://10.221.40.35:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"
