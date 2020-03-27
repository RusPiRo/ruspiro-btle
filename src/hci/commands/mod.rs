/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/
//! # Host Controller Interface Commands
//!
//! Each HCI command is something that could be thought on by parts of the brain to get a conclusion
//! on it. This part of the brain is actually the bluetooth low energy adaptor built into the
//! Raspberry Pi
//!

use super::errors::*;
use super::events::*;
use super::packet::*;
use super::*;
use crate::alloc::{boxed::Box, collections::BTreeMap, sync::Arc};
use crate::brain::*;
use crate::hctl::HcTransportLayer;
use crate::mem::size_of;
use crate::pin::Pin;
use crate::pin_utils::*;

mod reset;
pub use reset::*;
mod downloadminidriver;
pub use downloadminidriver::*;
mod writeclassofdevice;
pub use writeclassofdevice::*;
mod writelocalname;
pub use writelocalname::*;
mod writescanenable;
pub use writescanenable::*;
mod vendorbcm;
pub use vendorbcm::*;
mod inquiry;
pub use inquiry::*;
mod acceptconnection;
pub use acceptconnection::*;

const LINK_COMMANDS: u16 = 0x1 << 10;
const BASEBAND_COMMANDS: u16 = 0x03 << 10;
const INFORMATION_COMMANDS: u16 = 0x4 << 10;
const VENDOR_COMMANDS: u16 = 0x3F << 10;

#[repr(u16)]
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone)]
pub enum HciCommand {
    Unknown = 0x0,
    // OGF_LINK_CONTROL
    Inquiry = LINK_COMMANDS | 0x01,
    CreateConnection = LINK_COMMANDS | 0x05,
    AcceptConnection = LINK_COMMANDS | 0x09,

    // OGF_CONTROL_BASEBAND
    Reset = BASEBAND_COMMANDS | 0x03,
    WriteLocalName = BASEBAND_COMMANDS | 0x13,
    WriteScanEnable = BASEBAND_COMMANDS | 0x1A,
    WriteClassOfDevice = BASEBAND_COMMANDS | 0x24,

    // OGF_INFO_COMMANDS
    ReadVersionInfo = INFORMATION_COMMANDS | 0x01,
    ReadBDAddr = INFORMATION_COMMANDS | 0x09,

    // OGF_VENDOR_COMMANDS
    DownloadMiniDriver = VENDOR_COMMANDS | 0x2E,
    WriteRam = VENDOR_COMMANDS | 0x4C,
    LaunchRam = VENDOR_COMMANDS | 0x4E,
}

impl From<u16> for HciCommand {
    fn from(orig: u16) -> Self {
        match orig {
            _ if orig == HciCommand::Inquiry as u16 => HciCommand::Inquiry,
            _ if orig == HciCommand::CreateConnection as u16 => HciCommand::CreateConnection,
            _ if orig == HciCommand::AcceptConnection as u16 => HciCommand::AcceptConnection,
            _ if orig == HciCommand::Reset as u16 => HciCommand::Reset,
            _ if orig == HciCommand::WriteClassOfDevice as u16 => HciCommand::WriteClassOfDevice,
            _ if orig == HciCommand::WriteScanEnable as u16 => HciCommand::WriteScanEnable,
            _ if orig == HciCommand::WriteLocalName as u16 => HciCommand::WriteLocalName,
            _ if orig == HciCommand::DownloadMiniDriver as u16 => HciCommand::DownloadMiniDriver,
            _ if orig == HciCommand::WriteRam as u16 => HciCommand::WriteRam,
            _ if orig == HciCommand::LaunchRam as u16 => HciCommand::LaunchRam,
            _ => HciCommand::Unknown,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct HciCommandHeader {
    op_code: HciCommand,
    param_length: u8,
}

const fn get_command_size<T>() -> u8 {
    (size_of::<T>() - size_of::<HciCommandHeader>()) as u8
}

pub trait IsHciCommand: Sized + core::fmt::Debug {
    fn op_code(&self) -> HciCommand;
    fn size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Send a command asynchronously to the BT host controller.
/// It takes a [HciCommand] and the required Host Controller Interface
/// using a specific Transport Layer
pub fn send_command<C, T>(command: C, hci: Arc<DataLock<Hci<T>>>) -> impl Thinkable<Output = Result<(), BoxError>>
    where
        C: commands::IsHciCommand,
        T: HcTransportLayer + 'static,
{
    SendCommandThinkable::new(command, hci)
}

struct SendCommandThinkable<C, T>
where
    C: commands::IsHciCommand,
    T: HcTransportLayer + 'static,
{
    hci: Arc<DataLock<Hci<T>>>,
    op_code: commands::HciCommand,
    packet: Option<packet::HciPacket<C>>,
}

impl<C, T> SendCommandThinkable<C, T>
where
    C: commands::IsHciCommand,
    T: HcTransportLayer,
{
    unsafe_unpinned!(packet: Option<packet::HciPacket<C>>);
    unsafe_unpinned!(hci: Arc<DataLock<Hci<T>>>);

    fn new(command: C, hci: Arc<DataLock<Hci<T>>>) -> Self {
        Self {
            hci,
            op_code: command.op_code(),
            packet: Some(packet::HciPacket {
                p_type: packet::HciPacketType::Command,
                p_data: command,
            }),
        }
    }
}

impl<C, T> Thinkable for SendCommandThinkable<C, T>
where
    C: commands::IsHciCommand,
    T: HcTransportLayer,
{
    type Output = Result<(), BoxError>;

    fn think(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        // if there is a packet set this has not been passed to the host
        if self.packet.is_some() {
            // as we are about to progress, get the waker that will be registered fot this thinkable
            let waker = cx.waker().clone();
            // check if the host controller is able to accept a command
            let accept_commands = self.hci.read().accept_commands.load(Ordering::Acquire);
            if accept_commands >= 1 {
                self.hci
                    .read()
                    .accept_commands
                    .fetch_sub(1, Ordering::SeqCst);
                info!("send to host {:?}", self.packet);
                // the host accepts this packet, so send it
                let op_code = self.op_code;
                let packet = self.as_mut().packet().take().unwrap();
                let mut hci = self.as_mut().hci().lock();
                hci.command_response.insert(op_code, (waker, None));
                if let Some(ref mut transport) = hci.transport_layer {
                    let _ = transport.send_packet(unsafe { packet.as_array_ref() });
                }
                Conclusion::Pending
            } else {
                info!("no more packets free to send to the host");
                // we need to wait until we can send this packet to the host.
                // register our wakre to wake this thinkable once data has been received as than
                // we might be able to send this packet
                //self.hci().lock().command_to_send_waker.push(waker);
                Conclusion::Pending
            }
        } else {
            let op_code = self.op_code;
            // being here means we have send the packet and data has been received
            // so read the data, it should be the corresponding response?
            let mut hci = self.hci().lock();
            // get the response assigned to the opcode from the receiver thinkable
            if let Some(response) = hci.command_response.get_mut(&op_code) {
                // if there has been a response assigned
                if let Some(event) = response.1.take() {
                    // we are done based on the response. If it was a CommandComplete event
                    // we are done, if it is a CommandStatus event it depenmds on the returned status
                    match events::HciEventCommandComplete::try_from(event) {
                        Ok(complete) => {
                            // the completion of a command also indicates how many commands will be
                            // accepted from the host now
                            hci.accept_commands
                                .store(complete.num_cmd_packets, Ordering::Release);
                            // remove the waker for this command
                            hci.command_response.remove(&op_code);
                            Conclusion::Ready(Ok(()))
                        }
                        Err(event) => match events::HciEventCommandStatus::try_from(event) {
                            Ok(status) => {
                                if status.status == 0x00 {
                                    // the status of a command also indicates how many commands will be
                                    // accepted from the host now
                                    hci.accept_commands
                                        .store(status.num_cmd_packets, Ordering::Release);
                                    // remove the waker for this command
                                    hci.command_response.remove(&op_code);
                                    Conclusion::Ready(Ok(()))
                                } else {
                                    warn!("cmd {:?} failed with status {:?}", op_code, status.status);
                                    Conclusion::Ready(Err(Box::new(HciError {})))
                                }
                            }
                            Err(_) => Conclusion::Pending,
                        },
                    }
                } else {
                    // we got waken but there was no response assigned to our command so let's get
                    // waken again, the waker keeps registered and needs no re-registration
                    Conclusion::Pending
                }
            } else {
                // well we got waken but there is nothing registered for this command to wait for
                // the incoming data, where is this coming from ?
                error!("Unknown reason for beeing woken");
                unimplemented!();
            }
            /*
            // in case ther was no packet for us let's get woken with the next receiving..
            let waker = cx.waker().clone();
            self.hci().lock().command_response_waker.push(waker);

            // pending for the time beeing
            Conclusion::Pending
            */
        }
    }
}
