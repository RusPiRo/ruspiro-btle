/*************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Device Inquiry thoughts and functions
//! 

use super::*;

const INQUIRY_LAP_LIAC: [u8; 3] = [0x00, 0x8B, 0x9E];   // Limited dedicated Inquiry Access Code
const INQUIRY_LAP_GIAC: [u8; 3] = [0x33, 0x8B, 0x9E];	// General unlimited Inquiry Access Code

pub struct InquireDevicesThought {
    packet: Option<HciPacket<HciCommandInquiry>>,
    transport: SharedTransportLayer,
    state: InquiryState,
}

#[derive(Copy, Clone, Debug)]
enum InquiryState {
    Initial,
    Initiated,
    Running,
    Done,
}

impl InquireDevicesThought {
    pub fn new(transport: SharedTransportLayer) -> Self {
        Self {
            packet: Some(
                HciPacket {
                    p_type: HciPacketType::Command,
                    p_data: HciCommandInquiry::new(INQUIRY_LAP_GIAC, InquiryLength::Sec(3), 5)
                }
            ),
            transport,
            state: InquiryState::Initial,
        }
    }
}

impl Thinkable for InquireDevicesThought {
    type Output = (); // shall be the list of found devices?

    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        match this.state {
            InquiryState::Initial => {
                // initial state means we need to send the InquiryCommand
                let waker = cx.waker().clone();
                let packet = this.packet.take().unwrap();
                this.transport.take_for(|trans| trans.send_waking_packet(packet, waker));
                this.state = InquiryState::Initiated;
                Conclusion::Pending
            },
            InquiryState::Initiated => {
                // we have initiated the inquiry so wait for the command completion, this is than
                // followed by the actual inquiry result events...
                let response = this.transport.take_for(|trans| trans.recv_response());
                if let Some(response) = response {
                    // inquiry successfully started, so we are pending and need to register our waker
                    // to be waken once the results are received
                    let waker = cx.waker().clone();
                    this.transport.take_for(|trans| trans.set_waker(waker));
                    this.state = InquiryState::Running;
                    Conclusion::Pending
                } else {
                    // no successful inquiry start - we are done but with an empty list....
                    Conclusion::Ready(())
                }
            },
            InquiryState::Running => {
                // running state means we are waiting for inquiry results. Those could be up to a 
                // define maximum number of responses or a "time-out"
                // so once we got waken here check for the packet we have received
                let response = this.transport.take_for(|trans| trans.recv_response());
                if let Some(response) = response {
                    match HciEventInquiryResponse::try_from(response) {
                        Ok(result) => info!("Inquiry Result {:#X?}", result),
                        Err(data) => match HciEventInquiryComplete::try_from(data) {
                            Ok(complete) => info!("Inquiry Complete {:#X?}", complete),
                            Err(_) => error!("Unexpected inquiry response"),
                        },
                    };
                    let waker = cx.waker().clone();
                    this.transport.take_for(|trans| trans.set_waker(waker));
                    Conclusion::Pending
                } else {
                    // we got waken but there is no response... are we done ?
                    Conclusion::Ready(())

                }
            },
            InquiryState::Done => Conclusion::Ready(()),
        }
    }
}