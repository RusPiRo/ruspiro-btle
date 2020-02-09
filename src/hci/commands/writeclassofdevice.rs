/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI ClassOfDevice Command
//!

use super::{HciCommand, HciCommandHeader, IsHciCommand};

const CLASS_SIZE: usize = 3;

#[repr(C, packed)]
#[derive(Debug)]
pub struct HciCommandWriteClassOfDevice {
    header: HciCommandHeader,
    device_class: [u8; CLASS_SIZE],
}

impl HciCommandWriteClassOfDevice {
    pub fn new(device_class: [u8; CLASS_SIZE]) -> Self {
        Self {
            header: HciCommandHeader {
                op_code: HciCommand::WriteClassOfDevice,
                param_length: device_class.len() as u8,
            },
            device_class,
        }
    }
}

impl IsHciCommand for HciCommandWriteClassOfDevice {
    fn op_code(&self) -> HciCommand {
        self.header.op_code
    }
}
