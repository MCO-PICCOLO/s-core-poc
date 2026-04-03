/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::etcd;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the service URL for testing
    env::set_var("ROCKSDB_SERVICE_URL", "http://localhost:47007");

    println!("🧪 Testing gRPC RocksDB Service...");

    // Give the service a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Test health check
    println!("\n📋 Testing health check...");
    match etcd::health_check().await {
        Ok(is_healthy) => {
            println!(
                "✅ Health check successful: {}",
                if is_healthy { "healthy" } else { "not healthy" }
            );
        }
        Err(e) => {
            println!("❌ Health check failed: {}", e);
            return Ok(());
        }
    }

    // Test PUT operation
    println!("\n📝 Testing PUT operation...");
    match etcd::put("test_key", "test_value").await {
        Ok(()) => println!("✅ PUT operation successful"),
        Err(e) => println!("❌ PUT operation failed: {}", e),
    }

    // Test GET operation
    println!("\n📖 Testing GET operation...");
    match etcd::get("test_key").await {
        Ok(value) => println!("✅ GET operation successful: {}", value),
        Err(e) => println!("❌ GET operation failed: {}", e),
    }

    // Test batch PUT operation
    println!("\n📦 Testing batch PUT operation...");
    let items = vec![
        ("batch_key1".to_string(), "batch_value1".to_string()),
        ("batch_key2".to_string(), "batch_value2".to_string()),
    ];
    match etcd::batch_put(items).await {
        Ok(()) => println!("✅ Batch PUT operation successful"),
        Err(e) => println!("❌ Batch PUT operation failed: {}", e),
    }

    // Test get_all_with_prefix operation
    println!("\n🔍 Testing get_all_with_prefix operation...");
    match etcd::get_all_with_prefix("batch_").await {
        Ok(kvs) => {
            println!(
                "✅ Get all with prefix successful: found {} items",
                kvs.len()
            );
            for (key, value) in kvs {
                println!("  📄 {}: {}", key, value);
            }
        }
        Err(e) => println!("❌ Get all with prefix failed: {}", e),
    }

    // Test DELETE operation
    println!("\n🗑️  Testing DELETE operation...");
    match etcd::delete("test_key").await {
        Ok(()) => println!("✅ DELETE operation successful"),
        Err(e) => println!("❌ DELETE operation failed: {}", e),
    }

    println!("\n🎉 All tests completed!");

    Ok(())
}
