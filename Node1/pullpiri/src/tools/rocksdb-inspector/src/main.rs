/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use clap::{Arg, Command};
use rocksdb::{IteratorMode, Options, DB};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("RocksDB Inspector")
        .version("1.0")
        .about("Inspects RocksDB data stored by Pullpiri")
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .value_name("PATH")
                .help("Path to RocksDB directory")
                .default_value("/tmp/pullpiri_shared_rocksdb"),
        )
        .arg(
            Arg::new("prefix")
                .short('f')
                .long("prefix")
                .value_name("PREFIX")
                .help("Filter keys by prefix"),
        )
        .arg(
            Arg::new("key")
                .short('k')
                .long("key")
                .value_name("KEY")
                .help("Get specific key value"),
        )
        .arg(
            Arg::new("stats")
                .short('s')
                .long("stats")
                .help("Show database statistics")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("test")
                .short('t')
                .long("test")
                .help("Run helloworld data verification test")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let db_path = matches.get_one::<String>("path").unwrap();

    println!("🔍 RocksDB Inspector");
    println!("📁 Database path: {}", db_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Open RocksDB in read-only mode to avoid lock conflicts
    let mut opts = Options::default();
    opts.create_if_missing(false); // Don't create if doesn't exist

    let db = match DB::open_for_read_only(&opts, db_path, false) {
        Ok(db) => db,
        Err(e) => {
            println!("❌ Failed to open RocksDB at {}: {}", db_path, e);
            println!("💡 Make sure the database exists and helloworld.sh has been executed");
            return Ok(());
        }
    };

    if matches.get_flag("stats") {
        show_database_stats(&db)?;
        return Ok(());
    }

    if let Some(key) = matches.get_one::<String>("key") {
        get_specific_key(&db, key)?;
        return Ok(());
    }

    if matches.get_flag("test") {
        run_helloworld_test(&db).await?;
        return Ok(());
    }

    if let Some(prefix) = matches.get_one::<String>("prefix") {
        show_keys_with_prefix(&db, prefix)?;
    } else {
        show_all_data(&db)?;
    }

    Ok(())
}

fn show_database_stats(db: &DB) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Database Statistics:");

    // Get basic stats
    if let Ok(Some(stats)) = db.property_value("rocksdb.stats") {
        println!("{}", stats);
    }

    // Count total keys
    let mut total_keys = 0;
    let iter = db.iterator(IteratorMode::Start);
    for item in iter {
        let _result = item?;
        total_keys += 1;
    }

    println!("📈 Total Keys: {}", total_keys);

    Ok(())
}

fn get_specific_key(db: &DB, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔑 Getting key: {}", key);

    match db.get(key.as_bytes())? {
        Some(value) => {
            let value_str = String::from_utf8_lossy(&value);
            println!("✅ Found:");
            println!("   Key: {}", key);
            println!("   Value Length: {} bytes", value.len());
            println!("   Value: {}", value_str);

            // Try to pretty print if it's JSON
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&value_str) {
                println!("   JSON Pretty:");
                println!("{}", serde_json::to_string_pretty(&json_value)?);
            }
        }
        None => {
            println!("❌ Key not found: {}", key);
        }
    }

    Ok(())
}

fn show_keys_with_prefix(db: &DB, prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Keys with prefix: {}", prefix);

    let mut found_keys = Vec::new();
    let iter = db.iterator(IteratorMode::Start);

    for item in iter {
        let (key_bytes, value_bytes) = item?;
        let key = String::from_utf8_lossy(&key_bytes);

        if key.starts_with(prefix) {
            let value = String::from_utf8_lossy(&value_bytes);
            found_keys.push((key.to_string(), value.to_string()));
        }
    }

    if found_keys.is_empty() {
        println!("❌ No keys found with prefix: {}", prefix);
    } else {
        println!("✅ Found {} keys:", found_keys.len());
        for (key, value) in found_keys {
            println!("   📄 {}: {} bytes", key, value.len());
            if value.len() < 200 {
                println!("      {}", value);
            } else {
                println!("      {}...", &value[..200]);
            }
            println!();
        }
    }

    Ok(())
}

fn show_all_data(db: &DB) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 All stored data:");

    let mut data_by_category = HashMap::new();
    let iter = db.iterator(IteratorMode::Start);

    for item in iter {
        let (key_bytes, value_bytes) = item?;
        let key = String::from_utf8_lossy(&key_bytes);
        let value = String::from_utf8_lossy(&value_bytes);

        let category = if key.starts_with("cluster/") {
            "🏗️  Cluster"
        } else if key.starts_with("nodes/") {
            "🖥️  Nodes"
        } else if key.starts_with("Scenario/") {
            "📋 Scenarios"
        } else if key.starts_with("Package/") {
            "📦 Packages"
        } else if key.starts_with("Model/") {
            "🎯 Models"
        } else if key.starts_with("/metrics/") {
            "📊 Metrics"
        } else if key.starts_with("/logs/") {
            "📝 Logs"
        } else {
            "❓ Other"
        };

        data_by_category
            .entry(category)
            .or_insert_with(Vec::new)
            .push((key.to_string(), value.to_string()));
    }

    if data_by_category.is_empty() {
        println!("❌ No data found in RocksDB");
        println!("💡 Try running helloworld.sh first to populate data");
    } else {
        for (category, items) in data_by_category {
            println!("\n{} ({} items):", category, items.len());
            for (key, value) in items {
                println!("   📄 {}: {} bytes", key, value.len());
                if value.len() < 100 {
                    println!("      {}", value);
                } else {
                    println!("      {}...", &value[..100]);
                }
            }
        }
    }

    Ok(())
}

async fn run_helloworld_test(db: &DB) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running Helloworld Data Verification Test");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut test_results = Vec::new();

    // Test 1: Check if node registration exists
    let node_keys = ["cluster/nodes/yh", "nodes/yh"];
    for key in &node_keys {
        match db.get(key.as_bytes())? {
            Some(value) => {
                let value_str = String::from_utf8_lossy(&value);
                test_results.push((format!("✅ Node key: {}", key), true));
                println!("   Found: {} ({} bytes)", key, value.len());
                if value_str.contains("yh") || value_str.contains("10.231") {
                    println!("   ✅ Contains expected node information");
                }
            }
            None => {
                test_results.push((format!("❌ Node key missing: {}", key), false));
            }
        }
    }

    // Test 2: Check if helloworld scenario exists
    let scenario_key = "Scenario/helloworld";
    match db.get(scenario_key.as_bytes())? {
        Some(value) => {
            let value_str = String::from_utf8_lossy(&value);
            test_results.push(("✅ Helloworld scenario stored".to_string(), true));
            println!("   Found: {} ({} bytes)", scenario_key, value.len());

            // Check for expected content
            if value_str.contains("helloworld") && value_str.contains("idle") {
                println!("   ✅ Contains expected scenario data");
            }
        }
        None => {
            test_results.push(("❌ Helloworld scenario missing".to_string(), false));
        }
    }

    // Test 3: Check if package information exists
    let package_key = "Package/helloworld";
    match db.get(package_key.as_bytes())? {
        Some(value) => {
            let value_str = String::from_utf8_lossy(&value);
            test_results.push(("✅ Helloworld package stored".to_string(), true));
            println!("   Found: {} ({} bytes)", package_key, value.len());

            if value_str.contains("helloworld") {
                println!("   ✅ Contains expected package data");
            }
        }
        None => {
            test_results.push(("❌ Helloworld package missing".to_string(), false));
        }
    }

    // Test 4: Check if model information exists
    let model_key = "Model/helloworld";
    match db.get(model_key.as_bytes())? {
        Some(value) => {
            let value_str = String::from_utf8_lossy(&value);
            test_results.push(("✅ Helloworld model stored".to_string(), true));
            println!("   Found: {} ({} bytes)", model_key, value.len());

            if value_str.contains("helloworld") {
                println!("   ✅ Contains expected model data");
            }
        }
        None => {
            test_results.push(("❌ Helloworld model missing".to_string(), false));
        }
    }

    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Test Summary:");

    let passed_tests = test_results.iter().filter(|(_, passed)| *passed).count();
    let total_tests = test_results.len();

    for (result, _) in &test_results {
        println!("   {}", result);
    }

    println!(
        "\n🎯 Overall Result: {}/{} tests passed",
        passed_tests, total_tests
    );

    if passed_tests == total_tests {
        println!("🎉 All tests passed! Helloworld.sh data is properly stored in RocksDB");
    } else {
        println!("⚠️  Some tests failed. Consider running helloworld.sh again");
    }

    Ok(())
}
