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
use crate::{HciPacket, HciEventType};

/// The CommandComplete event will be send/received if the processing of a command has been finished
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct HciEventInquiryComplete {
    header: HciEventHeader,
    status: u8
}

impl TryFrom<HciPacket<Vec<u8>>> for HciEventInquiryComplete {
    type Error = HciPacket<Vec<u8>>;

    fn try_from(orig: HciPacket<Vec<u8>>) -> Result<Self, Self::Error> {
        let raw_event = orig.p_data;
        if raw_event[0] == HciEventType::InquiryComplete as u8 {
            Ok(
                HciEventInquiryComplete {
                    header: HciEventHeader {
                        evt_code: raw_event[0].into(),
                        param_length: raw_event[1],
                    },
                    status: raw_event[2],
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
