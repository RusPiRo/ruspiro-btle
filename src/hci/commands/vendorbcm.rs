/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # HCI Vendor BCM Command
//!

use super::{HciCommand, HciCommandHeader, IsHciCommand};

#[repr(C, packed)]
//#[derive(Copy, Clone)]
pub struct HciCommandVendorBcm {
    header: HciCommandHeader,
    data: [u8; 256],
}

impl HciCommandVendorBcm {
    pub fn new(op_code: HciCommand, data: &[u8]) -> Self {
        let mut command = Self {
            header: HciCommandHeader {
                op_code,
                param_length: data.len() as u8,
            },
            data: [0; 256],
        };

        command.data[..data.len()].copy_from_slice(data);
        command
    }
}

impl IsHciCommand for HciCommandVendorBcm {
    fn op_code(&self) -> HciCommand {
        self.header.op_code
    }

    fn size(&self) -> usize {
        core::mem::size_of::<HciCommandHeader>() + self.header.param_length as usize
    }
}

impl core::fmt::Debug for HciCommandVendorBcm {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "HciCommandVendorBcm {{ header: {{ {:?} }}, data: {{ size: {} }} }}",
            self.header,
            self.data.len()
        )
    }
}
