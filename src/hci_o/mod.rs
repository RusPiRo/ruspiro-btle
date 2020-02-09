/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # Bluetooth Host Controller Interface
//! The HostController Interface is the low level communication API from the bluetooth
//! using application to the underlying transport protokoll.
//!
use core::convert::TryFrom;
use core::pin::Pin;
use crate::alloc::{
    boxed::Box,
    sync::Arc,
};
use ruspiro_brain::*;
use ruspiro_singleton::Singleton;
use ruspiro_console::*;

mod commands;
pub use commands::*;
mod events;
pub use events::*;
mod packets;
pub use packets::*;

mod transport;
pub use transport::*;

mod firmware;
mod inquiry;
mod connection;

const HCI_CLASS_DESKTOP_COMPUTER: [u8; 3] = [0x04, 0x21, 0x00];
const HCI_CLASS_PS3_CONTROLLER: [u8; 3]   = [0x3E, 0x02, 0x04];

pub struct Hci {
    pub transport: SharedTransportLayer,
}

impl Hci {
    /// Create a new HCI instance. This will create the [TransportLayer] that is using the
    /// [Uart0] bridge to the BT Host for the low level communications
    pub fn new() -> Pin<Box<Self>> {
        Box::pin(
            Hci {
                transport: Arc::new(Singleton::new(TransportLayer::new())),
            }
        )
    }

    /// Initialize the Host Controller Interface using the transport layer implementation given
    /// as parameter. It is assumed the transportlayer is properly initialized and ready to be used
    pub fn initialize(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = Result<(), &'static str>> {
        let arc_self = self;
        let self1 = arc_self.clone();
        let self2 = arc_self.clone();
        let self3 = arc_self.clone();
        let self4 = arc_self.clone();
        match TransportLayer::initialize(arc_self.transport.clone()) {
            Err(e) => Left(ready(Err("something went wrong"))),
            Ok(_) => Right(
                arc_self.reset()
                    .then(|_| self1.upload_fw())
                    .then(|_| self2.set_class_of_device())
                    .then(|_| self3.set_local_name())
                    .then(|_| self4.set_scan_enable(ScanEnableType::Both))
                    .map(|_| {
                        info!("initializing done");
                        Ok(())
                    })
                ),
        }
    }

    /// Resetting the BT Host
    fn reset(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = ()> {
        use ruspiro_console::*;
        info!("reset the BT Host");
        SendPacketThought::from_command(
            HciCommandReset::new(),
            self.transport.clone()
        ).map(|response| {
            // if the command was processed the response typically provides a hint from the BT Host
            // how many packets it can now accept. We might need to store this information somewhere
            // to pospone HCI packets from being send if ther is no more acceptable
            match HciEventCommandComplete::try_from(response) {
                Ok(completed) => info!("Reset done {:?}", completed),
                Err(data) => error!("Unexpected response {:?}", data)
            };
        })
    }

    /// upload the firmware to the BTHost to enable a fully functionable device
    fn upload_fw(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = ()> {
        let transport1 = self.transport.clone();
        let transport2 = self.transport.clone();
        SendPacketThought::from_command(
            HciCommandDownloadMiniDriver::new(),
            transport1
        )
        .then(|response| {
            print!("upload firmware .");
            firmware::UploadFWThought::new(transport2)
        })
        .then(|_| {
            info!("wait for re-boot...");
            wait(Mseconds(500), ())
        })
        .map(|_| info!("fw upload done?"))
    }

    fn set_class_of_device(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = ()> {
        SendPacketThought::from_command(
            HciCommandWriteClassOfDevice::new(
                [0x04, 0x21, 0x00]
            ),
            self.transport.clone()
        ).map(|response| {
            // if the command was processed the response typically provides a hint from the BT Host
            // how many packets it can now accept. We might need to store this information somewhere
            // to pospone HCI packets from being send if ther is no more acceptable
            match HciEventCommandComplete::try_from(response) {
                Ok(completed) => info!("Set ClassOfDevice done {:?}", completed),
                Err(data) => error!("Unexpected response {:?}", data)
            };
        })
    }

    fn set_local_name(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = ()> {
        SendPacketThought::from_command(
            HciCommandWriteLocalName::new(b"RusPiRo"),
            self.transport.clone()
        ).map(|response| {
            // if the command was processed the response typically provides a hint from the BT Host
            // how many packets it can now accept. We might need to store this information somewhere
            // to pospone HCI packets from being send if ther is no more acceptable
            match HciEventCommandComplete::try_from(response) {
                Ok(completed) => info!("Set LocalName done {:?}", completed),
                Err(data) => error!("Unexpected response {:?}", data)
            };
        })
    }

    fn set_scan_enable(self: Arc<Pin<Box<Self>>>, scan_type: ScanEnableType) -> impl Thinkable<Output = ()> {
        SendPacketThought::from_command(
            HciCommandWriteScanEnable::new(scan_type),
            self.transport.clone(),
        ).map(|response| {
            // if the command was processed the response typically provides a hint from the BT Host
            // how many packets it can now accept. We might need to store this information somewhere
            // to pospone HCI packets from being send if ther is no more acceptable
            match HciEventCommandComplete::try_from(response) {
                Ok(completed) => info!("Set ScanEnable done {:?}", completed),
                Err(data) => error!("Unexpected response {:?}", data)
            };
        })
    }

    pub fn search_devices(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = ()> {
        info!("scan for devices...");
        inquiry::InquireDevicesThought::new(
            self.transport.clone()
        ).map(|response| {
            info!("inquiry done with {:#?}", response);
            // if the command was processed the response typically provides a hint from the BT Host
            // how many packets it can now accept. We might need to store this information somewhere
            // to pospone HCI packets from being send if ther is no more acceptable
            /*match HciEventCommandComplete::try_from(response) {
                Ok(completed) => info!("Start Inquiry done {:?}", completed),
                Err(data) => error!("Unexpected response {:?}", data)
            };*/
        })
    }

    pub fn handle_connection_requests(self: Arc<Pin<Box<Self>>>) -> impl Thinkable<Output = ()> {
        info!("handle incomming requests");
        connection::HandleInboundConnectionsThought::new(
            self.transport.clone()
        )
    }
}

