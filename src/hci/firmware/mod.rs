/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Firmware related Thoughts
//!

use super::commands::{HciCommand, HciCommandVendorBcm, SendCommandThinkable};
use super::errors::*;
use super::packet::*;
use super::*;
use crate::hctl::HcTransportLayer;
use crate::pin::Pin;

// TODO: check for alignment requirements on this external data
static FIRMWARE: &'static [u8] = include_bytes!("./BCM4345C0.hcd"); //PI 3 B+
                                                                    //static FIRMWARE: &'static [u8] = include_bytes!("./BCM43430A1.hcd"); //PI 2/3

pub struct UploadFirmwareThinkable<T>
where
    T: HcTransportLayer + 'static,
{
    hci: Arc<DataLock<Hci<T>>>,
    fw_offset: usize,
    command: SendCommandThinkable<HciCommandVendorBcm, T>,
}

impl<T> UploadFirmwareThinkable<T>
where
    T: HcTransportLayer,
{
    unsafe_unpinned!(hci: Arc<DataLock<Hci<T>>>);
    unsafe_unpinned!(fw_offset: usize);
    unsafe_unpinned!(command: SendCommandThinkable<HciCommandVendorBcm, T>);

    pub fn new(hci: Arc<DataLock<Hci<T>>>) -> Self {
        // calculate initial command to be send from firmware binary upload blob
        let op_code: HciCommand = (FIRMWARE[0] as u16 | (FIRMWARE[1] as u16) << 8).into();
        let chunk_size = FIRMWARE[2];
        let chunk_end = 3 + chunk_size as usize;
        // update the offset to point to the next chunk in the FW blob
        let fw_offset = chunk_end;

        Self {
            hci: hci.clone(),
            fw_offset,
            command: SendCommandThinkable::new(
                HciCommandVendorBcm::new(op_code, &FIRMWARE[3..chunk_end]),
                hci,
            ),
        }
    }

    // get the next packet to upload the fw blob to the BT Host
    fn next_command(&mut self) -> HciCommandVendorBcm {
        let op_code: HciCommand =
            (FIRMWARE[self.fw_offset] as u16 | (FIRMWARE[self.fw_offset + 1] as u16) << 8).into();
        let chunk_size = FIRMWARE[self.fw_offset + 2];
        let chunk_start = self.fw_offset + 3;
        let chunk_end = chunk_start + chunk_size as usize;
        // update the offset to point to the next chunk in the FW blob
        self.fw_offset += (chunk_size + 3) as usize;

        HciCommandVendorBcm::new(op_code, &FIRMWARE[chunk_start..chunk_end])
    }
}

impl<T> Thinkable for UploadFirmwareThinkable<T>
where
    T: HcTransportLayer,
{
    type Output = Result<(), BoxError>;

    fn think(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        let command = self.as_mut().command();
        let mut pin_command = Box::pin(command);
        match pin_command.as_mut().think(cx) {
            //self.as_mut().command().think(cx) {
            Conclusion::Pending => Conclusion::Pending,
            Conclusion::Ready(_) => {
                // if the upload command finished send the next chunk of firmware as long as
                // not all has been send
                if *self.as_mut().fw_offset() < FIRMWARE.len() {
                    let next_command = self.as_mut().next_command();
                    let hci_clone = self.as_ref().hci.clone();
                    *self.as_mut().command() = SendCommandThinkable::new(next_command, hci_clone);
                    // wake my self
                    cx.waker().wake_by_ref();
                    Conclusion::Pending
                } else {
                    Conclusion::Ready(Ok(()))
                }
            }
        }
    }
}
