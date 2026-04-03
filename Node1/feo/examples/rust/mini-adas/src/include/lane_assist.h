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

#ifndef MINIADAS_LANE_ASIST_H_
#define MINIADAS_LANE_ASIST_H_

#include <cstdint>

class LaneAssist {

  public:
    explicit LaneAssist(uint64_t activity_id);

    void startup();
    void step();
    void shutdown();

  private:
    uint64_t activity_id;
};

#endif
