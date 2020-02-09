/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # HCI Connection Request Event
//!

use core::convert::TryFrom;
use super::HciEventHeader;
use crate::alloc::vec::Vec;
use crate::{HciPacket, HciEventType};

/// The CommandComplete event will be send/received if the processing of a command has been finished
#[repr(C, packed)]
#[derive(Debug)]
pub struct HciEventConnectionRequest {
    header: HciEventHeader,
    address: [u8; 6],
    device_class: [u8; 3],
    link_type: u8,
}

impl TryFrom<HciPacket<Vec<u8>>> for HciEventConnectionRequest {
    type Error = HciPacket<Vec<u8>>;

    fn try_from(orig: HciPacket<Vec<u8>>) -> Result<Self, Self::Error> {
        let raw_event = orig.p_data;
        if raw_event[0] == HciEventType::ConnectionRequest as u8 {
            let mut event = HciEventConnectionRequest {
                header: HciEventHeader {
                    evt_code: raw_event[0].into(),
                    param_length: raw_event[1],
                },
                address: [0; 6],
                device_class: [0; 3],
                link_type: raw_event[10],
            };
            event.address.copy_from_slice(&raw_event[2..8]);
            event.device_class.copy_from_slice(&raw_event[8..11]);
            Ok(event)
        } else {
            Err(
                HciPacket {
                    p_type: orig.p_type,
                    p_data: raw_event,
                }
            )
        }
    }
}
