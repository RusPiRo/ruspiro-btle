/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # Title goes here
//!

use crate::alloc::boxed::Box;
use crate::alloc::collections::BTreeMap;
use crate::alloc::sync::Arc;
use crate::alloc::vec::Vec;
use crate::brain::{waker::*, *};
use crate::convert::TryFrom;
use crate::error::BoxError;
use crate::hctl::*;
use crate::lock::*;
use crate::pin::Pin;
use crate::pin_utils::*;
use crate::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use crate::{error, warn, info};

pub mod commands;
pub mod packet;
use packet::HciPacketType;
pub mod events;
use events::HciEventType;
pub mod errors;
mod firmware;
mod inquiry;
pub mod connection;

mod init;
use init::*;

/// Byte size of a bluetooth device address 
pub const BD_ADDRESS_SIZE: usize = 6;
/// Byte size of a bluetooth device "class of device" value
pub const BD_COD_SIZE: usize = 3;

/// Class Of Device for a computer/desktop device that is limited discoverable
/// As to the fact that the data need to be passed in little-endian to the BT Host
/// the array should be read backwards to get the "normal" hex value 0x002104
pub const COD_COMPUTER: [u8; BD_COD_SIZE] = [0x0C, 0x01, 0x02]; 

pub struct Hci<T: HcTransportLayer + 'static> {
    transport_layer: Option<Box<T>>,
    accept_commands: AtomicU8, //Semaphore,
    recv_waker: Option<Waker>,
    //command_response_waker: Vec<Waker>,
    //command_to_send_waker: Vec<Waker>,
    command_response: BTreeMap<commands::HciCommand, (Waker, Option<packet::HciPacket<Vec<u8>>>)>,
    event_notify: BTreeMap<events::HciEventType, (Waker, Option<packet::HciPacket<Vec<u8>>>)>,
}

impl<T: HcTransportLayer + 'static> Hci<T> {
    /// Create a new HostControllerInterface that is safe and shareable accross cores and Thinkables
    pub fn new(mut transport_layer: T) -> Arc<DataLock<Self>> {
        let hci = Arc::new(DataLock::new(Self {
            transport_layer: None,
            // initially the host accepts 1 packet at a time
            accept_commands: AtomicU8::new(1), //Semaphore::new(1),
            recv_waker: None,
            //command_response_waker: Vec::new(),
            //command_to_send_waker: Vec::new(),
            command_response: BTreeMap::new(),
            event_notify: BTreeMap::new(),
        }));

        let hci_clone = hci.clone();
        transport_layer.register_evt_handler(HctlEvent::Receive, move || {
            if let Some(ref waker) = hci_clone.read().recv_waker {
                waker.wake_by_ref();
            }
        });

        hci.lock()
            .transport_layer
            .replace(Box::new(transport_layer));

        hci
    }

    pub fn initialize(this: Arc<DataLock<Self>>) -> impl Thinkable<Output = ()> {
        init::HciInitThinkable::new(this)
            .map(|_| info!("initialization done"))
    }

    /// This returns the HCI ``Thinkable`` serving handling for any incoming data from the
    /// bluetooth host. This should be spawned to the Brain before any interaction with the bluetooth
    /// host can start.
    #[must_use]
    pub fn serve(this: Arc<DataLock<Self>>) -> impl Thinkable<Output = ()> {
        info!("start receiver thinkable");
        RecvPacketThinkable::new(this)
    }

    pub fn serve_connections(this: Arc<DataLock<Self>>) -> impl Thinkable<Output = ()> {
        info!("start connections thinkable");
        connection::HandleInboundConnectionsThinkable::new(this)
    }

    pub fn reset(this: Arc<DataLock<Self>>) -> impl Thinkable<Output = Result<(), BoxError>> {
        Self::send_command(this, commands::HciCommandReset::new())
    }

    pub fn upload_firmware(
        this: Arc<DataLock<Self>>,
    ) -> impl Thinkable<Output = Result<(), BoxError>> {
        let this_1 = this.clone();

        Self::send_command(this, commands::HciCommandDownloadMiniDriver::new())
            .then(|_| firmware::UploadFirmwareThinkable::new(this_1))
            .then(|_| wait::<Result<(), BoxError>>(Mseconds(500), Ok(())))
    }

    pub fn set_class_of_device(
        this: Arc<DataLock<Self>>,
        cod: [u8; 3],
    ) -> impl Thinkable<Output = Result<(), BoxError>> {
        Self::send_command(this, commands::HciCommandWriteClassOfDevice::new(cod))
    }

    pub fn set_local_name(
        this: Arc<DataLock<Self>>,
        name: &'static [u8],
    ) -> impl Thinkable<Output = Result<(), BoxError>> {
        Self::send_command(this, commands::HciCommandWriteLocalName::new(name))
    }

    pub fn set_scan_enable(
        this: Arc<DataLock<Self>>,
        scan_enable: commands::ScanEnableType,
    ) -> impl Thinkable<Output = Result<(), BoxError>> {
        Self::send_command(this, commands::HciCommandWriteScanEnable::new(scan_enable))
    }

    pub fn scan_devices(
        this: Arc<DataLock<Self>>,
    ) -> impl Thinkable<Output = Result<Vec<events::HciEventInquiryResponseData>, BoxError>> {
        inquiry::InquireDevicesThinkable::new(this, commands::InquiryLength::Sec(5))
    }

    /// Send a HCI Command packet to the host controller. This operation finishes if the Host
    /// either response with a CommandComplete or a CommandStatus event
    pub fn send_command<C>(
        this: Arc<DataLock<Self>>,
        command: C,
    ) -> impl Thinkable<Output = Result<(), BoxError>>
    where
        C: commands::IsHciCommand,
    {
        commands::SendCommandThinkable::new(command, this)
    }
}

/// This ``Thinkable`` will usually never conclude to a result as it will constantly be waken when
/// new data is received. Once done it "falls a sleep" again
struct RecvPacketThinkable<T>
where
    T: super::hctl::HcTransportLayer + 'static,
{
    hci: Arc<DataLock<Hci<T>>>,
    first: AtomicBool,
}

impl<T> RecvPacketThinkable<T>
where
    T: super::hctl::HcTransportLayer + 'static,
{
    unsafe_unpinned!(hci: Arc<DataLock<Hci<T>>>);

    fn new(hci: Arc<DataLock<Hci<T>>>) -> Self {
        Self {
            hci,
            first: AtomicBool::new(true),
        }
    }
}

impl<T> Thinkable for RecvPacketThinkable<T>
where
    T: super::hctl::HcTransportLayer,
{
    type Output = ();

    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        if self.first.load(Ordering::Relaxed) {
            // if this thinkable is triggered the first time this is not from a trigger that data is
            // available, so register our waker and wait for beeing woken
            self.first.store(false, Ordering::Relaxed);
            let hci: &Arc<DataLock<Hci<T>>> = &*self.hci();
            let mut hci = hci.lock();
            hci.recv_waker.replace(cx.waker().clone());
            info!("registered receive thinkable waker");
        } else {
            // at any other time we got here because data was received, so now start reading the
            // incomming data and trigger the corresponding processing
            let mut hci = self.hci().lock();
            if let Some(ref mut transport) = hci.transport_layer {
                // using a small buffer to be able to read the first important bytes from the BT Host
                let mut buff: [u8; 10] = [0; 10];
                // read 1 byte to begin with to the buffer. This contains the packet type we are about
                // to receive
                let _ = transport.recv_packet(&mut buff[..1]);
                // it has been seen that even if a packet has been fully received and a new one is
                // expected there are some arbitrary 0x0 values coming in from BT Host, so ignore them
                // and wait for a packet to begin
                if buff[0] != 0 {
                    // we are receiving a packet
                    let packet_type: HciPacketType = buff[0].into();
                    // read the next bytes beeing the content of this packet, however their
                    // meaning/format depends on the packet type
                    match packet_type {
                        HciPacketType::Event => {
                            // info!("received event");
                            // event packet: read the next 2 bytes containing the event type
                            // and the parameter size that need to be retreived additionaly
                            let _ = transport.recv_packet(&mut buff[1..3]);
                            let param_size = buff[2] as usize;
                            // create the generic buffer receiving the whole packet data
                            let mut packet_data = buff.to_vec();
                            // resize the buffer to be able to hold the data already read (3 bytes)
                            // and the parameter that are about to come
                            packet_data.resize(param_size + 3, 0);
                            let _ = transport.recv_packet(&mut packet_data[3..]);
                            // well we now have the whole packet received. Based on the event type
                            // we need to notify/wake the corresponding Thinkables that has registered
                            // themself to such an event packet
                            let event_type: HciEventType = packet_data[1].into();
                            match event_type {
                                HciEventType::CommandComplete => {
                                    // CommandComplete event, get the command that has been completed
                                    // to wake the right thinkable
                                    let command = commands::HciCommand::from(
                                        (packet_data[5] as u16) << 8 | packet_data[4] as u16,
                                    );
                                    // if we have a waker registered for this command
                                    // fill up the corresponding response and wake the waker
                                    if let Some(command_response) =
                                        hci.command_response.get_mut(&command)
                                    {
                                        command_response
                                            .1
                                            .replace(packet::HciPacket::from(packet_data));
                                        command_response.0.wake_by_ref();
                                    }
                                },
                                HciEventType::CommandStatus => {
                                    // CommandStatus event, get the command that has responded with
                                    // status and wake the right thinkable
                                    let command = commands::HciCommand::from(
                                        (packet_data[6] as u16) << 8 | packet_data[5] as u16,
                                    );
                                    // if we have a waker registered for this command
                                    // fill up the corresponding response and wake the waker
                                    if let Some(command_response) =
                                        hci.command_response.get_mut(&command)
                                    {
                                        command_response
                                            .1
                                            .replace(packet::HciPacket::from(packet_data));
                                        command_response.0.wake_by_ref();
                                    }
                                }
                                _ => {
                                    if let Some(event_notify) = hci.event_notify.get_mut(&event_type) {
                                        event_notify.1.replace(packet::HciPacket::from(packet_data));
                                        event_notify.0.wake_by_ref();
                                    } else {
                                        error!("event type {:?} doesn't notify anyone", event_type);
                                        unimplemented!()
                                    }
                                },
                            }
                        }
                        HciPacketType::Command => {
                            info!("received command");
                        }
                        HciPacketType::AclData => {
                            info!("received ACL Data");
                        }
                        _ => {
                            // well, what to do with an unknown packet type ?
                            // we might panic here as we cannot know how much data to read to
                            // stop where a new fresh known packet starts
                            error!("received packet: {:?} / raw: {}", packet_type, buff[0]);
                            panic!("can't handle unknown THost packet type");
                        }
                    }
                }
            }
        }

        Conclusion::Pending
    }
}
