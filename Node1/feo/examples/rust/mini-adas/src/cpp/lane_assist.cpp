/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#include "lane_assist.h"

#include "feo_cpp/feo_macros.h"
#include <cstdint>

LaneAssist::LaneAssist(const uint64_t activity_id) {
    this->activity_id = activity_id;
}

void LaneAssist::startup() {}

void LaneAssist::step() {}

void LaneAssist::shutdown() {}

// Create glue code for interface to Rust
MAKE_ACTIVITY(LaneAssist, lane_assist);

