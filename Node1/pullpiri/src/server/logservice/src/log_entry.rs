// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared log entry representation passed between the receiver and web UI.

use serde::Serialize;

/// A single log line broken into structured fields for filtering & rendering.
#[derive(Clone, Debug, Serialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: String,
    pub tag: String,
    pub message: String,
}
