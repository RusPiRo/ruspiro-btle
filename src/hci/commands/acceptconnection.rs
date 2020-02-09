/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI Accept Connection Command
//!

use crate::hci::BD_ADDRESS_SIZE;
use crate::hci::connection::HciConnectionRole;
use super::{get_command_size, HciCommand, HciCommandHeader, IsHciCommand};

#[derive(Debug)]
#[repr(C, packed)]
pub struct HciCommandAcceptConnection {
    header: HciCommandHeader,
    address: [u8; BD_ADDRESS_SIZE],
    role: HciConnectionRole,
}

impl HciCommandAcceptConnection {
    pub fn new(address: [u8; BD_ADDRESS_SIZE], role: HciConnectionRole) -> Self {
        Self {
            header: HciCommandHeader {
                op_code: HciCommand::AcceptConnection,
                param_length: get_command_size::<Self>(),
            },
            address,
            role
        }
    }
}

impl IsHciCommand for HciCommandAcceptConnection {
    fn op_code(&self) -> HciCommand {
        self.header.op_code
    }
}
