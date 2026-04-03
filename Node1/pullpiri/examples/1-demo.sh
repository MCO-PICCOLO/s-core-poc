#!/bin/bash

BODY1=$(< ./resources/face/face-1-pre.yaml)

curl -X POST 'http://10.0.0.30:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY1}"


BODY2=$(< ./resources/face/face-1.yaml)

#URL="10.0.0.30:8080/api/v1/yaml"

curl -X POST 'http://10.0.0.30:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY2}"
