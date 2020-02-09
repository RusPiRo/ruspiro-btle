/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # HCI Command Comletion Event
//!

use core::convert::TryFrom;
use super::HciEventHeader;
use crate::alloc::vec::Vec;
use crate::{HciCommand, HciPacket, HciEventType};
use ruspiro_console::*;

/// The CommandComplete event will be send/received if the processing of a command has been finished
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct HciEventCommandComplete {
    pub header: HciEventHeader,
    /// number of HCI commands allowed to be send after this event has been received
    pub num_cmd_packets: u8,
    /// Code of the completed command
    pub op_code: HciCommand,
    /// completeion status
    pub status: u8,
}

impl TryFrom<HciPacket<Vec<u8>>> for HciEventCommandComplete {
    type Error = HciPacket<Vec<u8>>;

    fn try_from(orig: HciPacket<Vec<u8>>) -> Result<Self, Self::Error> {
        let raw_event = orig.p_data;
        if raw_event[0] == HciEventType::CommandComplete as u8 {
            Ok(
                HciEventCommandComplete {
                    header: HciEventHeader {
                        evt_code: raw_event[0].into(),
                        param_length: raw_event[1],
                    },
                    num_cmd_packets: raw_event[2],
                    op_code: (raw_event[3] as u16 | (raw_event[4] as u16) << 8).into(),
                    status: raw_event[5],
                }
            )
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
