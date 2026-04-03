/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Diagnostic utilities for service connectivity

use common::logd;
use std::time::Duration;
use tokio::net::TcpStream;

/// Check if a service is reachable at the given IP and port
pub async fn check_service_connectivity(ip: &str, port: u16) -> bool {
    let addr = format!("{}:{}", ip, port);
    logd!(2, "Checking connectivity to {}", addr);

    match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => {
            logd!(2, "Successfully connected to {}", addr);
            true
        }
        Ok(Err(e)) => {
            logd!(4, "Failed to connect to {}: {}", addr, e);
            false
        }
        Err(_) => {
            logd!(4, "Connection timeout to {}", addr);
            false
        }
    }
}

/// Check if NodeAgent is reachable at the given IP
pub async fn check_node_agent_connectivity(ip: &str) -> bool {
    check_service_connectivity(ip, 47004).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use tokio::net::TcpListener as TokioTcpListener;
    use tokio::time::Duration;

    /// Helper function to start a mock TCP server for testing
    async fn start_mock_server(port: u16) -> Result<TokioTcpListener, std::io::Error> {
        let addr = format!("127.0.0.1:{}", port);
        TokioTcpListener::bind(addr).await
    }

    /// Helper function to find an available port for testing
    fn find_available_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    #[tokio::test]
    async fn test_check_service_connectivity_success() {
        // Start a mock server on an available port
        let port = find_available_port();
        let _listener = start_mock_server(port).await.unwrap();

        // Test successful connection
        let result = check_service_connectivity("127.0.0.1", port).await;
        assert!(result, "Should successfully connect to mock server");
    }

    #[tokio::test]
    async fn test_check_service_connectivity_connection_refused() {
        // Test connection to a port where no service is running
        let port = find_available_port(); // Get an available port but don't start a server
        let result = check_service_connectivity("127.0.0.1", port).await;
        assert!(!result, "Should fail to connect to non-existent service");
    }

    #[tokio::test]
    async fn test_check_service_connectivity_invalid_ip() {
        // Test connection to invalid IP address
        let result = check_service_connectivity("999.999.999.999", 8080).await;
        assert!(!result, "Should fail to connect to invalid IP address");
    }

    #[tokio::test]
    async fn test_check_service_connectivity_localhost_variations() {
        // Test different localhost representations
        let port = find_available_port();
        let _listener = start_mock_server(port).await.unwrap();

        // Test with 127.0.0.1
        let result1 = check_service_connectivity("127.0.0.1", port).await;
        assert!(result1, "Should connect to 127.0.0.1");

        // Test with localhost (may not work in all environments)
        let result2 = check_service_connectivity("localhost", port).await;
        // Note: This might fail in some test environments, so we just test it doesn't panic
        // In a real environment, localhost should resolve to 127.0.0.1
        logd!(2, "Localhost connection result: {}", result2);
    }

    #[tokio::test]
    async fn test_check_service_connectivity_timeout_simulation() {
        // Test with an IP that should cause timeout (non-routable IP)
        // Using 10.254.254.254 which is typically non-routable
        let result = check_service_connectivity("10.254.254.254", 12345).await;
        assert!(!result, "Should timeout and return false");
    }

    #[tokio::test]
    async fn test_check_service_connectivity_various_ports() {
        // Test with different port numbers
        let port = find_available_port();
        let _listener = start_mock_server(port).await.unwrap();

        // Test with the actual port
        let result1 = check_service_connectivity("127.0.0.1", port).await;
        assert!(result1, "Should connect to correct port");

        // Test with a different port (should fail)
        let wrong_port = if port == 65535 { port - 1 } else { port + 1 };
        let result2 = check_service_connectivity("127.0.0.1", wrong_port).await;
        assert!(!result2, "Should fail to connect to wrong port");
    }

    #[tokio::test]
    async fn test_check_service_connectivity_edge_case_ports() {
        // Test with edge case port numbers

        // Test with port 0 (should fail)
        let result1 = check_service_connectivity("127.0.0.1", 0).await;
        assert!(!result1, "Should fail to connect to port 0");

        // Test with high port number (but valid)
        let result2 = check_service_connectivity("127.0.0.1", 65534).await;
        assert!(
            !result2,
            "Should fail to connect to non-listening high port"
        );
    }

    #[tokio::test]
    async fn test_check_node_agent_connectivity_success() {
        // Test the NodeAgent specific function with a mock server on port 47004
        // Note: We can't easily bind to 47004 in tests as it might be in use
        // So we test the function call structure
        let result = check_node_agent_connectivity("127.0.0.1").await;
        // We don't assert the result since no actual NodeAgent is running
        // but we verify the function executes without panic
        logd!(2, "NodeAgent connectivity test result: {}", result);
    }

    #[tokio::test]
    async fn test_check_node_agent_connectivity_with_mock() {
        // Create a mock server on port 47004 if available
        match start_mock_server(47004).await {
            Ok(_listener) => {
                let result = check_node_agent_connectivity("127.0.0.1").await;
                assert!(result, "Should connect to mock NodeAgent on port 47004");
            }
            Err(_) => {
                // Port 47004 might be in use, test with unavailable service
                let result = check_node_agent_connectivity("127.0.0.1").await;
                // Just verify it doesn't panic - result depends on what's running on 47004
                logd!(
                    2,
                    "NodeAgent connectivity result (port might be in use): {}",
                    result
                );
            }
        }
    }

    #[tokio::test]
    async fn test_check_node_agent_connectivity_invalid_ip() {
        // Test NodeAgent connectivity with invalid IP
        let result = check_node_agent_connectivity("999.999.999.999").await;
        assert!(!result, "Should fail to connect NodeAgent at invalid IP");
    }

    #[tokio::test]
    async fn test_check_node_agent_connectivity_unreachable_ip() {
        // Test NodeAgent connectivity with unreachable IP
        let result = check_node_agent_connectivity("10.254.254.254").await;
        assert!(
            !result,
            "Should fail to connect NodeAgent at unreachable IP"
        );
    }

    #[tokio::test]
    async fn test_connectivity_functions_with_empty_ip() {
        // Test with empty IP string
        let result1 = check_service_connectivity("", 8080).await;
        assert!(!result1, "Should fail with empty IP string");

        let result2 = check_node_agent_connectivity("").await;
        assert!(!result2, "Should fail NodeAgent connectivity with empty IP");
    }

    #[tokio::test]
    async fn test_connectivity_timeout_behavior() {
        // Test that the timeout is actually working (should complete within reasonable time)
        let start = std::time::Instant::now();
        let result = check_service_connectivity("10.254.254.254", 12345).await;
        let duration = start.elapsed();

        assert!(!result, "Should return false for unreachable service");
        assert!(
            duration >= Duration::from_secs(3),
            "Should wait at least 3 seconds for timeout"
        );
        assert!(
            duration < Duration::from_secs(5),
            "Should not wait more than 5 seconds total"
        );
    }

    #[tokio::test]
    async fn test_multiple_concurrent_connectivity_checks() {
        // Test multiple concurrent connectivity checks
        let port = find_available_port();
        let _listener = start_mock_server(port).await.unwrap();

        let tasks = vec![
            tokio::spawn(check_service_connectivity("127.0.0.1", port)),
            tokio::spawn(check_service_connectivity("127.0.0.1", port)),
            tokio::spawn(check_service_connectivity("127.0.0.1", port)),
        ];

        let results = futures::future::join_all(tasks).await;

        for result in results {
            let connectivity_result = result.unwrap();
            assert!(
                connectivity_result,
                "All concurrent connections should succeed"
            );
        }
    }

    #[tokio::test]
    async fn test_connectivity_with_ipv6_localhost() {
        // Test with IPv6 localhost address
        let result = check_service_connectivity("::1", 8080).await;
        // This may fail in environments without IPv6 support, so we just ensure no panic
        logd!(2, "IPv6 localhost connectivity result: {}", result);
    }

    #[tokio::test]
    async fn test_service_connectivity_function_parameters() {
        // Test various parameter combinations to ensure robustness
        let test_cases = vec![
            ("127.0.0.1", 1),          // Low port number
            ("127.0.0.1", 65535),      // High port number
            ("0.0.0.0", 8080),         // All interfaces IP
            ("255.255.255.255", 8080), // Broadcast IP
        ];

        for (ip, port) in test_cases {
            let result = check_service_connectivity(ip, port).await;
            // We don't assert specific results as they depend on network configuration
            // but ensure no panics occur
            logd!(2, "Connectivity test for {}:{} = {}", ip, port, result);
        }
    }
}
