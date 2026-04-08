#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

BODY=$(< ./resources/safe-exit-assist.yaml)
#BODY=$(< ./resources/safe-exit-assist2.yaml)

## To terminate the artifact, use the same BODY content as above to identify it (e.g. by name).
#BODY=$(< ./resources/safe-exit-assist-stop.yaml)

curl -X POST 'http://10.221.40.35:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"

## To delete the artifact, use the same BODY content as above to identify it (e.g. by name).
# curl -X DELETE 'http://10.221.40.35:47099/api/artifact' \
# --header 'Content-Type: text/plain' \
# --data "${BODY}"