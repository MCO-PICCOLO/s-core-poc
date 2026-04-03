/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// TODO - add custom error message types
/*
pub struct Error {
    msg: Msg,
}

struct Msg {
    kind: ErrorKind,
    desc: Box<std::error::Error+Send+Sync>,
}

pub enum Errorkind {
    ...
}
*/
