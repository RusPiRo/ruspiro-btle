/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI ClassOfDevice Command
//!

use super::{get_command_size, HciCommand, HciCommandHeader, IsHciCommand};

const NAME_SIZE: usize = 248;

#[repr(u8)]
#[derive(Debug)]
pub enum ScanEnableType {
    None = 0x0,
    Inquiry = 0x1,
    Page = 0x2,
    Both = 0x3,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct HciCommandWriteScanEnable {
    header: HciCommandHeader,
    scan_type: ScanEnableType,
}

impl HciCommandWriteScanEnable {
    pub fn new(scan_type: ScanEnableType) -> Self {
        Self {
            header: HciCommandHeader {
                op_code: HciCommand::WriteScanEnable,
                param_length: get_command_size::<Self>(),
            },
            scan_type,
        }
    }
}

impl IsHciCommand for HciCommandWriteScanEnable {
    fn op_code(&self) -> HciCommand {
        self.header.op_code
    }
}
