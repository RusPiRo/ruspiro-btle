/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI ClassOfDevice Command
//!

use crate::alloc::vec::Vec;
use super::{get_command_size, HciCommand, HciCommandHeader, IsHciCommand};

const NAME_SIZE: usize = 248;

#[repr(C, packed)]
pub struct HciCommandWriteLocalName {
    header: HciCommandHeader,
    local_name: [u8; NAME_SIZE],
}

impl HciCommandWriteLocalName {
    pub fn new(local_name: &[u8]) -> Self {
        let mut command = Self {
            header: HciCommandHeader {
                op_code: HciCommand::WriteLocalName,
                param_length: local_name.len() as u8,
            },
            local_name: [0; NAME_SIZE],
        };

        command.local_name[..local_name.len()].copy_from_slice(local_name);
        command
    }
}

impl IsHciCommand for HciCommandWriteLocalName {
    fn op_code(&self) -> HciCommand {
        self.header.op_code
    }
    
    fn size(&self) -> usize {
        core::mem::size_of::<HciCommandHeader>() + self.header.param_length as usize
    }
}

impl core::fmt::Debug for HciCommandWriteLocalName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HciCommandWriteLocalName {{ header: {{ {:?} }}, data: {{ size: {} }} }}",
            self.header,
            self.local_name.len()
        )
    }
}