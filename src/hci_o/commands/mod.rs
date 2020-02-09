/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # Host Controller Interface Commands
//!
//! Each HCI command is something that could be thought on by parts of the brain to get a conclusion
//! on it. This part of the brain is actually the bluetooth low energy adaptor built into the
//! Raspberry Pi
//!

use crate::alloc::{sync::Arc, vec::Vec};
use crate::ruspiro_brain::*;
use crate::ruspiro_lock::DataLock;
use crate::{HciPacket, SharedTransportLayer};
use core::mem::size_of;
use core::pin::Pin;
use ruspiro_console::*;

mod reset;
pub use reset::*;
mod downloadminidriver;
pub use downloadminidriver::*;
mod writeclassofdevice;
pub use writeclassofdevice::*;
mod writelocalname;
pub use writelocalname::*;
mod writescanenable;
pub use writescanenable::*;
mod vendorbcm;
pub use vendorbcm::*;
mod inquiry;
pub use inquiry::*;

const LINK_COMMANDS: u16 = 0x1 << 10;
const BASEBAND_COMMANDS: u16 = 0x03 << 10;
const INFORMATION_COMMANDS: u16 = 0x4 << 10;
const VENDOR_COMMANDS: u16 = 0x3F << 10;

#[repr(u16)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
pub enum HciCommand {
    Unknown = 0x0,
    // OGF_LINK_CONTROL
    Inquiry = LINK_COMMANDS | 0x01,

    // OGF_CONTROL_BASEBAND
    Reset = BASEBAND_COMMANDS | 0x03,
    WriteLocalName = BASEBAND_COMMANDS | 0x13,
    WriteScanEnable = BASEBAND_COMMANDS | 0x1A,
    WriteClassOfDevice = BASEBAND_COMMANDS | 0x24,


    // OGF_INFO_COMMANDS
    ReadVersionInfo = INFORMATION_COMMANDS | 0x01,
    ReadBDAddr = INFORMATION_COMMANDS | 0x09,

    // OGF_VENDOR_COMMANDS
    DownloadMiniDriver = VENDOR_COMMANDS | 0x2E,
    WriteRam = VENDOR_COMMANDS | 0x4C,
    LaunchRam = VENDOR_COMMANDS | 0x4E,
}

impl From<u16> for HciCommand {
    fn from(orig: u16) -> Self {
        match orig {
            _ if orig == HciCommand::Inquiry as u16 => HciCommand::Inquiry,
            _ if orig == HciCommand::Reset as u16 => HciCommand::Reset,
            _ if orig == HciCommand::WriteClassOfDevice as u16 => HciCommand::WriteClassOfDevice,
            _ if orig == HciCommand::WriteScanEnable as u16 => HciCommand::WriteScanEnable,
            _ if orig == HciCommand::WriteLocalName as u16 => HciCommand::WriteLocalName,
            _ if orig == HciCommand::DownloadMiniDriver as u16 => HciCommand::DownloadMiniDriver,
            _ if orig == HciCommand::WriteRam as u16 => HciCommand::WriteRam,
            _ if orig == HciCommand::LaunchRam as u16 => HciCommand::LaunchRam,
            _ => HciCommand::Unknown,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct HciCommandHeader {
    op_code: HciCommand,
    param_length: u8,
}

const fn get_command_size<T>() -> u8 {
    (size_of::<T>() - size_of::<HciCommandHeader>()) as u8
}

pub trait IsHciCommand: Sized + core::fmt::Debug {
    fn op_code(&self) -> HciCommand;
    fn size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}
