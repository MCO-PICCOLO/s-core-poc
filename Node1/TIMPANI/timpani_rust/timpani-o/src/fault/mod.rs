/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Fault notification client for Pullpiri's `FaultService`.
//!
//! # Design vs C++ implementation
//!
//! The C++ used `FaultServiceClient::GetInstance()` — a Meyers singleton.
//! The singleton existed only to work around the C-style static callback
//! `DBusServer::DMissCallback`, which could not capture `this`.
//!
//! In the Rust port all callbacks are async closures with captured state,
//! so the singleton pattern is unnecessary.  `FaultClient` is injected as
//! `Arc<dyn FaultNotifier>` wherever it is needed.  This makes the component
//! testable without a live Pullpiri server.

use std::sync::Arc;

use thiserror::Error;
use tonic::transport::Channel;
use tracing::info;

use crate::proto::schedinfo_v1::{
    fault_service_client::FaultServiceClient as ProtoFaultClient, FaultInfo, FaultType,
};

// ── FaultNotification ─────────────────────────────────────────────────────────

/// Data carried in every fault notification sent to Pullpiri.
#[derive(Debug, Clone)]
pub struct FaultNotification {
    pub workload_id: String,
    pub node_id: String,
    pub task_name: String,
    pub fault_type: FaultType,
}

// ── FaultError ────────────────────────────────────────────────────────────────

/// Errors that can occur when notifying Pullpiri of a fault.
#[derive(Debug, Error)]
pub enum FaultError {
    /// tonic channel / endpoint construction failure.
    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// The gRPC call itself failed (network error, server unavailable, etc.).
    #[error("RPC status: {0}")]
    Rpc(#[from] tonic::Status),

    /// The RPC succeeded but Pullpiri returned a non-zero status code.
    #[error("Pullpiri returned non-zero status {0}")]
    RemoteError(i32),
}

// ── FaultNotifier trait ───────────────────────────────────────────────────────

/// Async interface for sending fault notifications to Pullpiri.
///
/// Implemented by [`FaultClient`] in production and by
/// [`test_support::MockFaultNotifier`] in tests.
#[tonic::async_trait]
pub trait FaultNotifier: Send + Sync {
    async fn notify_fault(&self, info: FaultNotification) -> Result<(), FaultError>;
}

// ── FaultClient ───────────────────────────────────────────────────────────────

/// Production gRPC client for Pullpiri's `FaultService`.
///
/// Created once at startup and shared via `Arc<dyn FaultNotifier>`.
pub struct FaultClient {
    // tonic client stubs are `Clone` — they share the underlying Arc<Channel>.
    // We clone on each call rather than wrapping in Mutex<...>.
    stub: ProtoFaultClient<Channel>,
}

impl FaultClient {
    /// Create a fault client that connects lazily to `addr`.
    ///
    /// The TCP connection is not established until the first RPC call.
    /// This avoids a hard startup ordering dependency on Pullpiri being live
    /// when Timpani-O starts.
    ///
    /// `addr` must be a full URI, e.g. `"http://localhost:50053"`.
    pub fn connect_lazy(addr: String) -> anyhow::Result<Arc<dyn FaultNotifier>> {
        let channel = tonic::transport::Endpoint::from_shared(addr)?.connect_lazy();
        let stub = ProtoFaultClient::new(channel);
        Ok(Arc::new(Self { stub }))
    }
}

#[tonic::async_trait]
impl FaultNotifier for FaultClient {
    async fn notify_fault(&self, info: FaultNotification) -> Result<(), FaultError> {
        let request = FaultInfo {
            workload_id: info.workload_id.clone(),
            node_id: info.node_id.clone(),
            task_name: info.task_name.clone(),
            r#type: info.fault_type as i32,
        };

        info!(
            workload_id = %info.workload_id,
            node_id     = %info.node_id,
            task_name   = %info.task_name,
            "Notifying Pullpiri of fault"
        );

        // Clone is cheap — Channel is Arc-backed.
        let mut stub = self.stub.clone();
        let response = stub
            .notify_fault(tonic::Request::new(request))
            .await?
            .into_inner();

        if response.status != 0 {
            return Err(FaultError::RemoteError(response.status));
        }

        Ok(())
    }
}

// ── Test support ──────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Mutex;

    /// A no-op `FaultNotifier` that records calls.
    ///
    /// Use in unit tests to assert that the correct fault notifications are
    /// generated without needing a live Pullpiri server.
    pub struct MockFaultNotifier {
        pub calls: Mutex<Vec<FaultNotification>>,
    }

    impl MockFaultNotifier {
        /// Returns a typed `Arc<Self>` so tests can inspect `.calls` directly.
        pub fn arc() -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
            })
        }
    }

    #[tonic::async_trait]
    impl FaultNotifier for MockFaultNotifier {
        async fn notify_fault(&self, info: FaultNotification) -> Result<(), FaultError> {
            tracing::debug!(
                task = %info.task_name,
                node = %info.node_id,
                "MockFaultNotifier: recording fault (suppressed in test)"
            );
            self.calls.lock().unwrap().push(info);
            Ok(())
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::test_support::MockFaultNotifier;
    use super::*;
    use crate::proto::schedinfo_v1::FaultType;

    fn make_notification(workload_id: &str) -> FaultNotification {
        FaultNotification {
            workload_id: workload_id.into(),
            node_id: "node01".into(),
            task_name: "task_safety".into(),
            fault_type: FaultType::Dmiss,
        }
    }

    #[tokio::test]
    async fn mock_notifier_records_single_call() {
        let notifier = MockFaultNotifier::arc();
        notifier
            .notify_fault(make_notification("wl1"))
            .await
            .unwrap();
        let calls = notifier.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].workload_id, "wl1");
        assert_eq!(calls[0].task_name, "task_safety");
    }

    #[tokio::test]
    async fn mock_notifier_records_multiple_calls() {
        let notifier = MockFaultNotifier::arc();
        for i in 0..5_u32 {
            notifier
                .notify_fault(make_notification(&format!("wl{i}")))
                .await
                .unwrap();
        }
        assert_eq!(notifier.calls.lock().unwrap().len(), 5);
    }

    #[test]
    fn fault_error_remote_error_display() {
        let e = FaultError::RemoteError(42);
        assert!(e.to_string().contains("42"));
    }

    #[test]
    fn fault_error_rpc_status_display() {
        let e = FaultError::Rpc(tonic::Status::not_found("msg"));
        assert!(!e.to_string().is_empty());
    }

    #[tokio::test]
    async fn fault_client_connect_lazy_valid_uri_succeeds() {
        FaultClient::connect_lazy("http://localhost:59999".to_string())
            .expect("valid URI should not fail");
    }
}
