/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Host Controller Interface Errors
//!

use crate::error::Error;

pub struct HciError {}

impl Error for HciError {}
//unsafe impl Send for HciError {}
//unsafe impl Sync for HciError {}

impl core::fmt::Display for HciError {
    /// Provide the human readable text for this error
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Error in Hci")
    }
}

impl core::fmt::Debug for HciError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // debug just calls the diplay implementation
        <HciError as core::fmt::Display>::fmt(self, f)
    }
}
