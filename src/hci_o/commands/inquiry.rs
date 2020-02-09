/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI Inquiry Command
//! This command is used to search for near by bluetooth devices

use super::{get_command_size, HciCommand, HciCommandHeader, IsHciCommand};

pub enum InquiryLength {
    Min, // minimum inquiry length 1.28s
    Max, // maximum inquiry length 61.44s
    Sec(u8),    // inquiry length in seconds: ((seconds) * 100 + 64) / 128
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct HciCommandInquiry {
    header: HciCommandHeader,
    lap: [u8; 3],
    length: u8,
    max_responses: u8,
}

impl HciCommandInquiry {
    pub fn new(lap: [u8; 3], length: InquiryLength, max_responses: u8) -> Self {
        Self {
            header: HciCommandHeader {
                op_code: HciCommand::Inquiry,
                param_length: get_command_size::<Self>(),
            },
            lap,
            length: length.into(),
            max_responses,
        }
    }
}

impl From<InquiryLength> for u8 {
    fn from(orig: InquiryLength) -> u8 {
        match orig {
            InquiryLength::Min => 0x01,
            InquiryLength::Max => 0x30,
            InquiryLength::Sec(s) => (((s as u16) * 100 + 64) / 128) as u8,
        }
    }
}

impl IsHciCommand for HciCommandInquiry {
    fn op_code(&self) -> HciCommand {
        self.header.op_code
    }
}
