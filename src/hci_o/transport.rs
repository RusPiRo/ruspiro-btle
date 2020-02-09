/***************************************************************************************************
 * Copyright (c) 2019 by the authors
 *
 * Author: André Borrmann
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Bluetooth Transport Layer
//! The transport layer is the lowest level communication implementation responsible to send data to
//! and receive data from the bluetooth host. On the Raspberry Pi the transport layer utilizes the
//! Uart0 serial interface to send the data.

use super::{packets::*, commands::*, events::*};
use crate::alloc::{vec::Vec, sync::Arc, collections::BTreeMap};
use crate::ruspiro_console::*;
use crate::ruspiro_singleton::Singleton;
use crate::ruspiro_uart::Uart0;
use crate::ruspiro_brain::{*, waker::Waker, mpmc::*};
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use pin_utils::{unsafe_unpinned};

pub type SharedTransportLayer = Arc<Singleton<TransportLayer>>;

/// Store the waker for a specific event type
struct HciWaker {
    waker: Waker,
    used: AtomicBool,
}

pub struct TransportLayer {
    /// The Uart communication instance.
    uart: Uart0,
    /// store a list of waker that shall be waken with a command status or completion event
    command_event_waker: BTreeMap<HciCommand, HciWaker>,
    /// The Waker that need to be waken once an event has been arrived from the BT Host
    event_waker: BTreeMap<HciEventType, HciWaker>,

    //waker: Option<Waker>,
    /// A multi producer/multi consumer queue storing the incomming packets
    inbound_channel: (Sender<HciPacket<Vec<u8>>>, Receiver<HciPacket<Vec<u8>>>),
    unhandled_inbound_packets: Vec<HciPacket<Vec<u8>>>,
}

impl TransportLayer {
    /// Create a new TransportLayer instance. This will ultimately also create a new Uart0
    /// instance. However, to be usable the [initialize] function need to be called
    pub(crate) fn new() -> Self {
        TransportLayer {
            uart: Uart0::new(),
            command_event_waker: BTreeMap::new(),
            event_waker: BTreeMap::new(),
            //waker: None,
            inbound_channel: channel(),
            unhandled_inbound_packets: Vec::new(),
        }
    }

    /// Initializing the ``TransportLayer`` sets up the Uart0 bridge and registers the interrupt
    /// handler to be called whenever the Uart0 receives data
    pub(crate) fn initialize(this: SharedTransportLayer) -> Result<(), &'static str> {
        // first initialize the Uart0 bridge to allow data send to the BT Host
        this.take_for(|transport| transport.uart.initialize(48_000_000, 115_200))?;
        // third register the function that shall be used when the Uart0 receives data from the BT Host
        // register the handler as closure, this will also activate the Uart0 interrupts beeing
        // dispatched
        let clone = this.clone();
        this.take_for(|transport| {
            transport
                .uart
                .register_irq_handler(move |_| TransportLayer::handle_incomming_data(&clone))
        });
        Ok(())
    }

    /// Send a packet to the BT Host storing the waker that need to be woken once the BT Host responds
    /// with data. 
    /// TODO: The assumption is that only one packet is send and waited for response at a time
    /// So the next received response is always the one the current Waker was "registered" for
    /// This assumption need to be verified or the Hci implementation need to ensure this contrain
    /// is met
    pub fn send_waking_packet<C: IsHciCommand + core::fmt::Debug>(&mut self, packet: HciPacket<C>, waker: Waker) {
        //self.waker.replace(waker);
        self.command_event_waker.insert(
            packet.p_data.op_code(),
            HciWaker { waker, used: AtomicBool::new(false) }
        );
        
        let data = unsafe { packet.as_array_ref() };
        self.uart.send_data(data);
    }

    /// Set's a new waker to be notified once a package has been recieved
    /// TODO: We might need to distinguesh the packet types and events we want the waker to be registered
    /// for!
    pub fn set_event_waker(&mut self, event_type: HciEventType, waker: Waker) {
        self.event_waker.insert(event_type, HciWaker {
            waker,
            used: AtomicBool::new(false)
        });
    }

    /// Retrieve data from the available BT Host responses for a specific packet type
    pub fn recv_response(&mut self, packet_type: HciPacketType) -> Option<HciPacket<Vec<u8>>> {
        // check the inbound channel for data that has been received
        while let Ok(packet) = self.inbound_channel.1.recv() {
            if packet.p_type == packet_type {
                return Some(packet);
            } else {
                self.unhandled_inbound_packets.push(packet);
            }
        }

        None
    }

    /// Handler to read the incomming data of the BT Host and rebuild a [HciPacket] from it that
    /// could be futher processed by the Host Controller Interface
    /// # HINT:
    /// This handler is running inside the interrupt context and shall not do "heavy" processing
    /// when calling back into the HCI implementation
    fn handle_incomming_data(this: &SharedTransportLayer) {
        // when new data has arrived, read the data and hand it over to the
        // Host Controller Interface for processing
        this.use_for(|transport: &TransportLayer| {
            // using a small buffer to be able to read the first important bytes from the BT Host
            let mut buff: [u8; 10] = [0; 10];
            // read 1 byte to begin with to the buffer
            let _ = transport.uart.receive_data(&mut buff[..1]);
            // it has been seen that even if a packet has been fully received and a new one is
            // expected there are some arbitrary 0x0 values coming in from BT Host, so ignore them
            // and wait for a packet to begin
            if buff[0] != 0 {
                // we are receiving a packet
                let packet_type: HciPacketType = buff[0].into();
                // read the next bytes beeing the content of this packet, however there
                // meaning/format depends on the packet type
                match packet_type {
                    HciPacketType::Event => {
                        // event packet: read the next 2 bytes containing the event type
                        // and the parameter size that need to be retreived additionaly
                        let _ = transport.uart.receive_data(&mut buff[1..3]);
                        let param_size = buff[2] as usize;
                        // create the generic buffer receiving the whole packet data
                        let mut packet_data = buff.to_vec();
                        // resize the buffer to be able to hold the data already read (3 bytes)
                        // and the parameter that are about to come
                        packet_data.resize(param_size + 3, 0);
                        let _ = transport.uart.receive_data(&mut packet_data[3..]);
                        //info!("recieved event - length {}", packet_data.len());
                        // get eventtype and possible command from the raw data
                        let event_type = packet_data[1];
                        let maybe_command = match event_type {
                            _ if event_type == HciEventType::CommandComplete as u8 => Some(packet_data[4] as u16 | (packet_data[5] as u16) << 8),
                            _ if event_type == HciEventType::CommandStatus as u8 => Some(packet_data[5] as u16 | (packet_data[6] as u16) << 8),
                            _ => None,
                        };
                        // so we have completely received a packet from BT Host. 
                        let packet = HciPacket::from(packet_data);
                        //info!("packet: {:#X?}", packet);
                        // push the packet to the channel
                        transport.inbound_channel.0.send(packet);
                        // if this is a command status or complete event trigger the appropriate waker
                        if event_type == HciEventType::CommandComplete as u8 ||
                           event_type == HciEventType::CommandStatus as u8 {
                            let op = maybe_command.unwrap();
                            let op_code = op.into();
                            if let Some(waker) = transport.command_event_waker.get(&op_code) {
                                waker.used.store(true, Ordering::Relaxed);
                                waker.waker.wake_by_ref();
                            } else {
                                info!("no waker for command done event {:#X?}", op);
                            }
                        } else {
                            // if it is not the generic command status/completed event check for the
                            // event type waker
                            //info!("received event {}", event_type);
                            if let Some(waker) = transport.event_waker.get(&event_type.into()) {
                                waker.used.store(true, Ordering::Relaxed);
                                waker.waker.wake_by_ref();
                            } else {
                                info!("no waker for event {:#X?}", event_type);
                            }
                        }
                    }
                    HciPacketType::Command => {
                        info!("received command");
                        // command packet: read the next 3 bytes containing the command (u16) and
                        // the parameter size that need to be retreived additionally
                        let _ = transport.uart.receive_data(&mut buff[1..4]);
                        let param_size = buff[3] as usize;
                        // create the generic buffer receiving the whole packet data
                        let mut packet_data = buff.to_vec();
                        // resize the buffer to be able to hold the data already read (4 bytes)
                        // and the parameter that are about to come
                        packet_data.resize(param_size + 4, 0);
                        let _ = transport.uart.receive_data(&mut packet_data[4..]);
                    }
                    HciPacketType::AclData => {
                        info!("received ACL Data");
                        // ACL data packet: read the next 4 bytes containing the ACL header info
                        // including the data payload size that need to be retreived
                        let _ = transport.uart.receive_data(&mut buff[1..5]);
                        // first two bytes are the control info and the following 2 the payload size
                        let payload_size = buff[3] as usize | (buff[4] as usize) << 8;
                        // create the generic buffer receiving the whole packet data
                        let mut packet_data = buff.to_vec();
                        // resize the buffer to be able to hold the data already read (4 bytes)
                        // and the parameter that are about to come
                        packet_data.resize(payload_size + 4, 0);
                        let _ = transport.uart.receive_data(&mut packet_data[4..]);
                    }
                    _ => {
                        // well, what to do with an unknown packet type ?
                        // we might panic here as we cannot know how much data to read to
                        // stop where a new fresh known packet starts
                        error!("received packet: {:?} / raw: {}", packet_type, buff[0]);
                        unimplemented!();
                    }
                };
                /*// so we have completely received a packet from BT Host. 
                let packet = HciPacket::from(raw_packet);
                // push the packet to the channel
                transport.inbound_channel.0.send(packet);
                // wake the waker that is waiting for data to be received
                if let Some(ref waker) = transport.waker {
                    waker.wake_by_ref();
                } else {
                    warn!("no waker for packet");
                }*/
            }
        });
    }
}

pub struct SendPacketThought<C> {
    packet: Option<HciPacket<C>>,
    transport: SharedTransportLayer,
}

impl<C> SendPacketThought<C> {
    unsafe_unpinned!(transport: SharedTransportLayer);
    unsafe_unpinned!(packet: Option<HciPacket<C>>);
    
    pub fn from_command(command: C, transport: SharedTransportLayer) -> Self {
        SendPacketThought {
            packet: Some(
                HciPacket {
                    p_type: HciPacketType::Command,
                    p_data: command,
                }
            ),
            transport,
        }
    }
}

impl<C> Thinkable for SendPacketThought<C>
    where C: IsHciCommand
{
    type Output = HciPacket<Vec<u8>>;

    fn think(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        if self.packet.is_some() {
            //info!("send packet to BT Host {:?}", self.packet);
            // this is the first time we think on this thought, so send the packet to the BT Host
            // passing the waker of this Thought to be woken as soon as there is a response from the
            // BT Host
            let waker = cx.waker().clone();
            let packet = self.as_mut().packet().take().unwrap();
            // need mutual exclusive access to the TansportLayer to ensure only one Thought at any
            // given time sends data to the BT Host
            self.as_mut().transport().take_for(|trans| {
                trans.send_waking_packet(packet, waker)
            });
            // indicate to the Brain that we need to think on this again, once the BT Host has
            // responded
            Conclusion::Pending
        } else {
            //info!("send packet thought woken, response there?");
            // this is the second time we think on this thought, so waiting for the response from
            // BT Host. As we were awoken to get here the BT Host has send a response so check with
            // the TransportLayer if this is 'our' response
            // need mutual exclusive access to the TansportLayer to check the buffer of data that has
            // been received so far
            let response = self.as_mut().transport().take_for(|trans| {
                // TODO: We assume only one packet could be send at any time and therefore only one
                // response is expected to arrive for exactly this packet that has been send
                trans.recv_response(HciPacketType::Event)
            });
            if let Some(response) = response {
                //info!("packet response {:?}", response);
                Conclusion::Ready(response)
            } else {
                // 'our' response is not yet there, so need to re-think this one. In this case there
                // is no need to pass the waker again to the TransportLayer as this will keep the 
                // reference until the corresponding response has been received
                Conclusion::Pending
            }
        }
    }
}