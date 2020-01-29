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

use crate::alloc::{sync::Arc, vec::Vec};
use crate::ruspiro_brain::*;
use crate::ruspiro_lock::DataLock;
use crate::{HciPacket, SharedTransportLayer};
use core::mem::size_of;
use core::pin::Pin;
use ruspiro_console::*;

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
    //fn op_code(&self) -> HciCommand;
    fn size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

struct HciCommandContext<CMD: IsHciCommand> {
    command: CMD,
    response: Option<HciPacket<Vec<u8>>>,
    stage: HciCommandStage,
    transport: SharedTransportLayer,
}

pub struct HciCommandThought<CMD: IsHciCommand, R> {
    context: Arc<DataLock<HciCommandContext<CMD>>>,
    _r: core::marker::PhantomData<R>,
}

enum HciCommandStage {
    Sending,
    Pending,
    Done,
}


/*
//impl<CMD: IsHciCommand> Unpin for HciCommandThought<CMD> {}

impl<CMD: IsHciCommand, R> HciCommandThought<CMD, R> {
    pub fn new(transport: SharedTransportLayer, command: CMD) -> Self {
        HciCommandThought {
            context: Arc::new(DataLock::new(HciCommandContext {
                command,
                response: None,
                stage: HciCommandStage::Sending,
                transport,
            })),
            _r: core::marker::PhantomData,
        }
    }
}

impl<CMD, R> Thought for HciCommandThought<CMD, R>
where
    CMD: IsHciCommand + 'static,
    R: From<HciPacket<Vec<u8>>>,
{
    type Output = R;

    /// Think on this HciCommand will go through the following stages:
    /// 1. Sending: when Thought the first time the command will be send to the BT Host using the
    ///     TransportLayer. As part of this the Though registers its waker to be waken once the BT Host
    ///     responds with the completion of the command processing.
    /// 2. Pending: when the Thought is woken up due to data received from the BT Host the response
    ///     will be taken and the Thought has come to a conclusion
    /// 3. Done: This indicates the thought was woken and was thought on after it has been already
    ///     come to a conclusion - this is treated as implementation error
    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        let command_ctx_clone = self.context.clone();
        let mut command_ctx = self.context.lock();
        match command_ctx.stage {
            HciCommandStage::Sending => {
                // send the command and pass a closure as call back that shall be called as soon as
                // the BT Host responds with data. This callback will store the response in the
                // Thought and wake the same
                command_ctx.transport.take_for(|trans| {
                    let waker = cx.waker().clone();
                    /*trans.send_packet(command_ctx.command.into(), move |response| {
                        let mut command_ctx = command_ctx_clone.lock();
                        command_ctx.response.replace(response);
                        drop(command_ctx);
                        waker.wake_by_ref();
                        println!("woken the thought");
                    });*/
                });
                // set the current stage of the Thought to pending
                command_ctx.stage = HciCommandStage::Pending;
                Conclusion::Pending
            }
            HciCommandStage::Pending => {
                if command_ctx.response.is_some() {
                    command_ctx.stage = HciCommandStage::Done;
                    println!("changed stage, response: {:X?}", command_ctx.response);
                    Conclusion::Ready(command_ctx.response.take().unwrap().into())
                } else {
                    Conclusion::Pending
                }
            }
            HciCommandStage::Done => unimplemented!(),
        }

        /*
        let (next_stage, conclusion) = match &this.stage {
            HciCommandStage::Sending => {
                let waker = cx.waker().clone();
                println!("Cmd: {:?}", this.command);
                let command = HciCommandReset::new();//this.command;
                let command = this.command.take().unwrap();
                println!("Cmd: {:?}", this.command);

                let response = this.response.clone();
                this.transport.take_for(|transport| transport.send_packet(
                    HciPacket::new_command(command),
                    move |cmd_response| {
                        response.take_for(|rsp| rsp.replace(cmd_response));
                        waker.wake_by_ref();
                    }
                ));

                (HciCommandStage::Pending, Conclusion::Pending)
            },
            HciCommandStage::Pending => {
                println!("command completed with {:?}", this.response.take_for(|resp| resp.take()));
                (HciCommandStage::Done, Conclusion::Ready(()))
            },
            HciCommandStage::Done => unimplemented!(),
        };

        this.stage = next_stage;
        conclusion
        */
    }
}
*/
