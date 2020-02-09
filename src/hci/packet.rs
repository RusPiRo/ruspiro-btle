/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # BT HCI Packet
//!

use super::commands::*;
use crate::alloc::vec::Vec;

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HciPacketType {
    Unknown = 0,
    Command = 1,
    AclData = 2,
    SyncData = 3,
    Event = 4,
}

impl core::convert::From<u8> for HciPacketType {
    fn from(orig: u8) -> HciPacketType {
        match orig {
            1 => HciPacketType::Command,
            2 => HciPacketType::AclData,
            3 => HciPacketType::SyncData,
            4 => HciPacketType::Event,
            _ => HciPacketType::Unknown,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct HciPacket<T> {
    pub(crate) p_type: HciPacketType,
    pub(crate) p_data: T,
}

impl<C: IsHciCommand> HciPacket<C> {
    pub const fn new(p_type: HciPacketType, content: C) -> Self {
        HciPacket {
            p_type,
            p_data: content,
        }
    }

    /// Get the [``HciPacket``] data as ``[u8]`` array reference.
    /// # Safety
    /// The size occupied by the contained data must not be less then the
    /// size returned by its ``size`` function.
    pub unsafe fn as_array_ref(&self) -> &[u8] {
        let raw_ptr = self as *const Self as *const u8;
        core::slice::from_raw_parts(raw_ptr, self.p_data.size() + 1)
    }
}

impl<C: IsHciCommand> From<C> for HciPacket<C> {
    fn from(orig: C) -> Self {
        HciPacket {
            p_type: HciPacketType::Command,
            p_data: orig,
        }
    }
}

impl From<Vec<u8>> for HciPacket<Vec<u8>> {
    fn from(orig: Vec<u8>) -> Self {
        HciPacket {
            p_type: orig[0].into(),
            p_data: orig[1..].to_vec(),
        }
    }
}
