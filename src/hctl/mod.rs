/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Host Controller Transport Layer
//!
//! Specification of the trait providing the Host Controller Transport Layer abstraction. This trait
//! need to be implemented by the actual transport layer to be used to communicate with the Bluetooth
//! host. On a Raspberry Pi this is usually the UART

use crate::error::BoxError;
use crate::uart::Uart0;

pub enum HctlEvent {
    Receive,
}

pub trait HcTransportLayer: Sync + Send {
    fn send_packet(&mut self, data: &[u8]) -> Result<usize, BoxError>;
    fn recv_packet(&mut self, buffer: &mut [u8]) -> Result<usize, BoxError>;
    fn register_evt_handler<F: FnMut() + 'static + Send>(&mut self, event: HctlEvent, function: F);
}

/// provide default implementation for UART0 used on Raspberry Pi
impl HcTransportLayer for Uart0 {
    fn send_packet(&mut self, data: &[u8]) -> Result<usize, BoxError> {
        self.send_data(data);
        Ok(data.len())
    }

    fn recv_packet(&mut self, buffer: &mut [u8]) -> Result<usize, BoxError> {
        self.receive_data(buffer)
    }

    fn register_evt_handler<F: FnMut() + 'static + Send>(
        &mut self,
        _event: HctlEvent,
        function: F,
    ) {
        self.register_irq_handler(function);
    }
}
