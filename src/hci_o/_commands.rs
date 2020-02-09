/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI Command Packets
//!

use core::mem::size_of;

const BASEBAND_COMMANDS: u16 = 0x03 << 10;
const INFORMATION_COMMANDS: u16 = 0x4 << 10;
const VENDOR_COMMANDS: u16 = 0x3F << 10;

#[repr(u16)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
pub enum HciCommand {
    Unknown = 0x0,
    // OGF_LINK_CONTROL

    // OGF_CONTROL_BASEBAND
    Reset = BASEBAND_COMMANDS | 0x03,

    // OGF_INFO_COMMANDS
    ReadVersionInfo = INFORMATION_COMMANDS | 0x01,
    ReadBDAddr = INFORMATION_COMMANDS | 0x09,

    // OGF_VENDOR_COMMANDS
    DownloadMiniDriver = VENDOR_COMMANDS | 0x2E,
    WriteRam = VENDOR_COMMANDS | 0x4C,
    LaunchRam = VENDOR_COMMANDS | 0x4E,
}

pub trait IsHciCommand {
    fn header(&self) -> &HciCommandHeader;
}

#[repr(C, packed)]
pub struct HciCommandHeader {
    op_code: u16,
    pub param_length: u8,
}

#[repr(C, packed)]
pub struct HciCommandReset {
    header: HciCommandHeader,
}

impl IsHciCommand for HciCommandReset {
    fn header(&self) -> &HciCommandHeader {
        &self.header
    }
}

impl HciCommandReset {
    pub fn new() -> Self {
        HciCommandReset {
            header: HciCommandHeader {
                op_code: HciCommand::Reset as u16,
                param_length: get_command_size::<Self>(),
            },
        }
    }
}

#[repr(C, packed)]
pub struct HciCommandDownloadMiniDriver {
    header: HciCommandHeader,
}

impl IsHciCommand for HciCommandDownloadMiniDriver {
    fn header(&self) -> &HciCommandHeader {
        &self.header
    }
}

impl HciCommandDownloadMiniDriver {
    pub fn new() -> Self {
        HciCommandDownloadMiniDriver {
            header: HciCommandHeader {
                op_code: HciCommand::DownloadMiniDriver as u16,
                param_length: get_command_size::<Self>(),
            },
        }
    }
}

#[repr(C, packed)]
pub struct HciCommandReadBDAddr {
    header: HciCommandHeader,
}

impl IsHciCommand for HciCommandReadBDAddr {
    fn header(&self) -> &HciCommandHeader {
        &self.header
    }
}

impl HciCommandReadBDAddr {
    pub fn new() -> Self {
        HciCommandReadBDAddr {
            header: HciCommandHeader {
                op_code: HciCommand::ReadBDAddr as u16,
                param_length: get_command_size::<Self>(),
            },
        }
    }
}

#[repr(C, packed)]
pub struct HciCommandVendorBcm {
    header: HciCommandHeader,
    data: [u8; 255],
}

impl IsHciCommand for HciCommandVendorBcm {
    fn header(&self) -> &HciCommandHeader {
        &self.header
    }
}

impl HciCommandVendorBcm {
    pub fn new(op_code: u16, param_length: u8, data: &[u8]) -> Self {
        let mut temp: [u8; 255] = [0; 255];
        temp[..data.len()].copy_from_slice(data);
        HciCommandVendorBcm {
            header: HciCommandHeader {
                op_code,
                param_length,
            },
            data: temp,
        }
    }
}

const fn get_command_size<T>() -> u8 {
    (size_of::<T>() - size_of::<HciCommandHeader>()) as u8
}
