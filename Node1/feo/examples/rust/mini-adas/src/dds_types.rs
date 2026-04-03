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

//! DDS message types published externally by the primary agent.
//!
//! These types are serialised via CDR (DDS wire format) and broadcast on
//! DDS domain 100.  They are intentionally separate from the FEO-internal
//! shared-memory types in `mini-adas-gen` so that neither the FEO signalling
//! path nor the DDS path depend on each other's serialisation format.

use dust_dds_derive::DdsType;
use serde::{Deserialize, Serialize};

/// Vehicle state broadcast on DDS topic `vehicle/state` every FEO cycle.
///
/// `gear`:
/// - `0` — Park (P): vehicle has been stationary for more than 5 seconds
/// - `1` — Drive (D): vehicle is moving or has just come to rest
#[derive(Debug, Default, Clone, Serialize, Deserialize, DdsType)]
pub struct VehicleState {
    pub speed: f32,
    pub gear: u8,
}

/// Rear-camera obstacle data broadcast on DDS topic `sensor/rear_camera`.
///
/// Only published when `VehicleState.speed < 5.0 km/h` to save bandwidth.
/// Subscribers (e.g., the Safe Exit app) use this to decide whether a
/// passenger can safely open a rear door.
#[derive(Debug, Default, Clone, Serialize, Deserialize, DdsType)]
pub struct RearCameraScan {
    /// `true` if an object is detected within a dangerous range behind the car.
    pub obstacle_detected: bool,
    /// Estimated distance to the closest object in metres.
    pub distance: f32,
}
