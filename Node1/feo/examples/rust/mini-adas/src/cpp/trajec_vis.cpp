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

#include "trajec_vis.h"

#include "feo_cpp/feo_macros.h"
#include <cstdint>

TrajectoryVisualizer::TrajectoryVisualizer(const uint64_t activity_id) {
    this->activity_id = activity_id;
}

void TrajectoryVisualizer::startup() {}

void TrajectoryVisualizer::step() {}

void TrajectoryVisualizer::shutdown() {}


// Create glue code for interface to Rust
MAKE_ACTIVITY(TrajectoryVisualizer, trajectory_visualizer);

