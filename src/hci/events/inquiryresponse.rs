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
#[derive(Debug)]
pub struct HciEventInquiryResponse {
    header: HciEventHeader,
    num_devices: u8,
    data: Vec<HciEventInquiryResponseData>,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct HciEventInquiryResponseData {
    address: [u8; 6],
    page_scan_repetition: u8,
    reserved: u16,
    class_of_device: [u8; 3],
    clock_offset: u16,
}

impl TryFrom<HciPacket<Vec<u8>>> for HciEventInquiryResponse {
    type Error = HciPacket<Vec<u8>>;

    fn try_from(orig: HciPacket<Vec<u8>>) -> Result<Self, Self::Error> {
        let raw_event = orig.p_data;
        if raw_event[0] == HciEventType::InquiryResult as u8 {
            // convert the binary payload into the structure
            let length = raw_event[2] as usize;
            let mut data = Vec::with_capacity(length);
            let data_ptr = &raw_event[3] as *const u8 as *const HciEventInquiryResponseData;
            for i in 0..length {
                let response_data = unsafe {
                    core::ptr::read_volatile(data_ptr.offset(i as isize))
                };
                data.push(response_data);
            }
            
            Ok(
                HciEventInquiryResponse {
                    header: HciEventHeader {
                        evt_code: raw_event[0].into(),
                        param_length: raw_event[1],
                    },
                    num_devices: raw_event[2],
                    data,
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
