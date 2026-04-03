/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
pub mod grpc;

fn main() {
    println!("Hello, world!");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
