/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # HCI Connection Request Event
//!

use crate::alloc::vec::Vec;
use crate::convert::TryFrom;
use crate::hci::events::{HciEventHeader, HciEventType};
use crate::hci::packet::HciPacket;
use crate::hci::connection::{
    HciConnectionLinkType,
    HciEncryptionType,
};

/// The CommandComplete event will be send/received if the processing of a command has been finished
#[repr(C, packed)]
#[derive(Debug)]
pub struct HciEventConnectionComplete {
    header: HciEventHeader,
    status: u8,
    handle: u16, //only the least 12Bit's uses
    address: [u8; 6],
    link_type: HciConnectionLinkType,
    encryption_mode: HciEncryptionType,
}

impl TryFrom<HciPacket<Vec<u8>>> for HciEventConnectionComplete {
    type Error = HciPacket<Vec<u8>>;

    fn try_from(orig: HciPacket<Vec<u8>>) -> Result<Self, Self::Error> {
        let raw_event = orig.p_data;
        if raw_event[0] == HciEventType::ConnectionComplete as u8 {
            let mut event = HciEventConnectionComplete {
                header: HciEventHeader {
                    evt_code: raw_event[0].into(),
                    param_length: raw_event[1],
                },
                status: raw_event[2],
                handle: raw_event[3] as u16 | (raw_event[4] as u16) << 8,
                address: [0; 6],
                link_type: raw_event[11].into(),
                encryption_mode: raw_event[12].into(),
            };
            event.address.copy_from_slice(&raw_event[5..11]);
            Ok(event)
        } else {
            Err(HciPacket {
                p_type: orig.p_type,
                p_data: raw_event,
            })
        }
    }
}
