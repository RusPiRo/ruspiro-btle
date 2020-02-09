/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # HCI Command Status Event
//!

use crate::alloc::vec::Vec;
use crate::convert::TryFrom;
use crate::hci::commands::HciCommand;
use crate::hci::events::{HciEventHeader, HciEventType};
use crate::hci::packet::HciPacket;

/// The CommandComplete event will be send/received if the processing of a command has been finished
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct HciEventCommandStatus {
    pub header: HciEventHeader,
    /// completeion status
    pub status: u8,
    /// number of HCI commands allowed to be send after this event has been received
    pub num_cmd_packets: u8,
    /// Code of the completed command
    pub op_code: HciCommand,
}

impl TryFrom<HciPacket<Vec<u8>>> for HciEventCommandStatus {
    type Error = HciPacket<Vec<u8>>;

    fn try_from(orig: HciPacket<Vec<u8>>) -> Result<Self, Self::Error> {
        let raw_event = orig.p_data;
        if raw_event[0] == HciEventType::CommandStatus as u8 {
            Ok(HciEventCommandStatus {
                header: HciEventHeader {
                    evt_code: raw_event[0].into(),
                    param_length: raw_event[1],
                },
                status: raw_event[2],
                num_cmd_packets: raw_event[3],
                op_code: (raw_event[4] as u16 | (raw_event[5] as u16) << 8).into(),
            })
        } else {
            Err(HciPacket {
                p_type: orig.p_type,
                p_data: raw_event,
            })
        }
    }
}
