/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI Reset Command
//!

use super::{get_command_size, HciCommand, HciCommandHeader, IsHciCommand};

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct HciCommandReset {
    header: HciCommandHeader,
}

impl HciCommandReset {
    pub fn new() -> Self {
        Self {
            header: HciCommandHeader {
                op_code: HciCommand::Reset,
                param_length: get_command_size::<Self>(),
            },
        }
    }
}

impl IsHciCommand for HciCommandReset {
    /*fn op_code(&self) -> HciCommand {
        self.header.op_code
    }*/
}
