/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use apiserver::manager;
use tokio::time::Duration;

#[tokio::test(flavor = "current_thread")]
async fn test_manager_initialize() {
    let _ = tokio::time::timeout(Duration::from_secs(5), manager::initialize()).await;
    assert!(true);
}
