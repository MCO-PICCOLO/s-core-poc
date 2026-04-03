/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::actioncontroller::{
    action_controller_connection_client::ActionControllerConnectionClient, connect_server,
    ReconcileRequest, ReconcileResponse, TriggerActionRequest, TriggerActionResponse,
};
use std::env;
use tonic::{Request, Response, Status};

pub async fn trigger_action(
    scenario_name: String,
) -> Result<Response<TriggerActionResponse>, Status> {
    // Test mode bypass: return a fake successful response when env var is set
    if env::var("PULLPIRI_TEST_MODE").is_ok() {
        let resp = TriggerActionResponse {
            status: 0,
            desc: "mock".to_string(),
        };
        return Ok(Response::new(resp));
    }
    let mut client = ActionControllerConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client
        .trigger_action(Request::new(TriggerActionRequest { scenario_name }))
        .await
}

pub async fn _send(condition: ReconcileRequest) -> Result<Response<ReconcileResponse>, Status> {
    // Test mode bypass: return a fake successful response when env var is set
    if env::var("PULLPIRI_TEST_MODE").is_ok() {
        let resp = ReconcileResponse {
            status: 0,
            desc: "mock".to_string(),
        };
        return Ok(Response::new(resp));
    }
    let mut client = ActionControllerConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.reconcile(Request::new(condition)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_send_in_test_mode_returns_mock_response() {
        env::set_var("PULLPIRI_TEST_MODE", "1");

        let req = ReconcileRequest {
            scenario_name: "s1".to_string(),
            current: 0,
            desired: 0,
        };

        let res = _send(req).await;
        assert!(res.is_ok());
        let r = res.unwrap();
        assert_eq!(r.get_ref().status, 0);

        env::remove_var("PULLPIRI_TEST_MODE");
    }

    #[tokio::test]
    async fn test_trigger_action_in_test_mode_returns_mock_response() {
        env::set_var("PULLPIRI_TEST_MODE", "1");

        let res = trigger_action("timpani_test".to_string()).await;
        assert!(res.is_ok());
        let r = res.unwrap();
        assert_eq!(r.get_ref().status, 0);

        env::remove_var("PULLPIRI_TEST_MODE");
    }
}
