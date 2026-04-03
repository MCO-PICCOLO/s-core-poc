/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! CLI-specific tests

// Note: These are basic unit tests for the CLI components
// Full CLI testing would require more sophisticated mocking

#[test]
fn test_error_display() {
    use pirictl::error::CliError;

    let error = CliError::Custom("test error".to_string());
    assert_eq!(format!("{}", error), "Error: test error");
}

#[test]
fn test_client_creation() {
    use pirictl::SettingsClient;

    let client = SettingsClient::new("http://localhost:47098", 30);
    assert!(client.is_ok());
}

// Note: More comprehensive CLI argument parsing tests would require
// exposing the CLI struct from main.rs or restructuring the code

/// Verify that two independent clients can be created with different URLs,
/// matching the dual-client (settings_client + api_client) routing design.
#[test]
fn test_dual_client_creation() {
    use pirictl::SettingsClient;

    let settings_client = SettingsClient::new("http://localhost:8080", 30);
    let api_client = SettingsClient::new("http://localhost:47099", 30);
    assert!(
        settings_client.is_ok(),
        "settings_client creation should succeed"
    );
    assert!(api_client.is_ok(), "api_client creation should succeed");
}
