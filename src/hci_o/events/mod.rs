/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # Host Controller Interface Events
//!
//! HCI Events are typically send from the Bluetooth Host to indicate that a requested command
//! execution has been finished
//!

use crate::alloc::vec::Vec;
mod commandcomplete;
pub use commandcomplete::*;
mod commandstatus;
pub use commandstatus::*;
mod inquiryresponse;
pub use inquiryresponse::*;
mod inquirycomplete;
pub use inquirycomplete::*;
mod connectionrequest;
pub use connectionrequest::*;

#[repr(u8)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
pub enum HciEventType {
    Unknown = 0x0,
    InquiryComplete = 0x1,
    InquiryResult = 0x2,
    ConnectionComplete = 0x3,
    ConnectionRequest = 0x4,
    DisconnectionComplete = 0x5,
    AuthenticationComplete = 0x6,
    RemoteNameRequestComplete = 0x7,
    CommandComplete = 0xE,
    CommandStatus = 0xF,
    RoleChange = 0x12,
    NumberOfCompletedPackets = 0x13,
    PinCodeRequest = 0x16,
    LinkKeyRequest = 0x17,
    LinkKeyNotification = 0x18,
    MaxSlotsChange = 0x1B,
}

impl From<u8> for HciEventType {
    fn from(orig: u8) -> Self {
        match orig {
            0x01 => HciEventType::InquiryComplete,
            0x02 => HciEventType::InquiryResult,
            0x03 => HciEventType::ConnectionComplete,
            0x04 => HciEventType::ConnectionRequest,
            0x05 => HciEventType::DisconnectionComplete,
            0x06 => HciEventType::AuthenticationComplete,
            0x07 => HciEventType::RemoteNameRequestComplete,
            0x0E => HciEventType::CommandComplete,
            0x0F => HciEventType::CommandStatus,
            0x12 => HciEventType::RoleChange,
            0x13 => HciEventType::NumberOfCompletedPackets,
            0x16 => HciEventType::PinCodeRequest,
            0x17 => HciEventType::LinkKeyRequest,
            0x18 => HciEventType::LinkKeyNotification,
            0x1B => HciEventType::MaxSlotsChange,
            _ => HciEventType::Unknown,
        }
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct HciEventHeader {
    pub evt_code: HciEventType,
    pub param_length: u8,
}

/*
/// Convert a byte array into the Event representation
impl<E: IsHciEvent> From<Vec<u8>> for E {
    fn from(orig: Vec<u8>) -> E {
        assert!(orig.len() == core::mem::size_of::<E>());
        let event = unsafe {
            let ptr = orig.as_ptr() as *const E;
            core::ptr::read_volatile(ptr)
        };
        // as we have re-castet the vector using a raw pointer to the target type we need to ensure
        // the original is not dropped
        core::mem::forget(orig);

        event
    }
}
*/
