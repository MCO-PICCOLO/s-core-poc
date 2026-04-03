/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Integration tests for pirictl

use pirictl::SettingsClient;

#[tokio::test]
async fn test_client_creation() {
    let client = SettingsClient::new("http://localhost:47098", 30);
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_client_creation_with_invalid_timeout() {
    let client = SettingsClient::new("http://localhost:47098", 0);
    assert!(client.is_ok()); // Client creation should succeed even with 0 timeout
}

#[tokio::test]
async fn test_health_check_with_unreachable_service() {
    // Use a port that's unlikely to be in use
    let client = SettingsClient::new("http://localhost:59999", 1).unwrap();
    let result = client.health_check().await;

    // Should return false or error when service is unreachable
    match result {
        Ok(false) => {} // Expected
        Err(_) => {}    // Also acceptable
        Ok(true) => panic!("Health check should not succeed for unreachable service"),
    }
}

// Note: The following tests require a running SettingsService
// They are commented out by default and can be enabled for integration testing

/*
#[tokio::test]
async fn test_metrics_endpoint_with_running_service() {
    let client = SettingsClient::new("http://localhost:47098", 30).unwrap();
    let result = client.get("/api/v1/metrics").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_boards_endpoint_with_running_service() {
    let client = SettingsClient::new("http://localhost:47098", 30).unwrap();
    let result = client.get("/api/v1/boards").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_nodes_endpoint_with_running_service() {
    let client = SettingsClient::new("http://localhost:47098", 30).unwrap();
    let result = client.get("/api/v1/nodes").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_socs_endpoint_with_running_service() {
    let client = SettingsClient::new("http://localhost:47098", 30).unwrap();
    let result = client.get("/api/v1/socs").await;
    assert!(result.is_ok());
}
*/
