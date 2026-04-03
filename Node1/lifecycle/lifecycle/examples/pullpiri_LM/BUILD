# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
load("//:defs.bzl", "launch_manager_config")

launch_manager_config(
    name = "pullpiri_config",
    config = "//examples/pullpiri_LM/config:pullpiri_lm_config",
)

sh_binary(
    name = "run_pullpiri",
    srcs = ["run.sh"],
    data = [
        ":pullpiri_config",
        "//src/launch_manager_daemon:launch_manager",
    ],
)
