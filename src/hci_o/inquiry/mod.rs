/*************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Device Inquiry thoughts and functions
//! 
use crate::alloc::vec::Vec;
use super::*;
use crate::hci::commands::*;
use crate::hctl::HcTransportLayer;
use crate::pin::Pin;

const INQUIRY_LAP_LIAC: [u8; 3] = [0x00, 0x8B, 0x9E];   // Limited dedicated Inquiry Access Code
const INQUIRY_LAP_GIAC: [u8; 3] = [0x33, 0x8B, 0x9E];	// General unlimited Inquiry Access Code

pub struct InquireDevicesThinkable<T>
where T: HcTransportLayer + 'static
{
    hci: Arc<DataLock<Hci<T>>>,
    state: InquiryState,
    command: SendCommandThinkable<HciCommandInquiry>,
    devices: Option<Vec<HciEventInquiryResponseData>>,
}

#[derive(Copy, Clone, Debug)]
enum InquiryState {
    Initial,
    Initiated,
    Running,
    Done,
}

impl<T> InquireDevicesThinkable<T>
where T: HcTransportLayer,
{
    unsafe_unpinned!(hci: Arc<DataLock<Hci<T>>>);
    unsafe_unpinned!(state: InquiryState);
    unsafe_unpinned!(command: SendCommandThinkable<HciCommandInquiry>);

    pub fn new(hci: Arc<DataLock<Hci<T>>>, length: InquiryLength) -> Self {
        Self {
            hci,
            state: InquiryState::Initial,
            command: HciCommandInquiry::new(INQUIRY_LAP_GIAC, InquiryLength::Sec(3), 5),
            devices: Some(Vec::new()),
        }
    }
}

impl<T> Thinkable for InquireDevicesThinkable<T>
where T: HcTransportLayer,
{
    type Output = Result<Vec<HciEventInquiryResponseData>, BoxError>;

    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        // let this = unsafe { self.get_unchecked_mut() };

        match this.state {
            InquiryState::Initial => {
                // initial state means we need to send the InquiryCommand
                let mut pin_command = Box::pin(self.as_mut().command());
                match pin_command.as_mut().think(cx) {
                    // sending the inquiry command is pending
                    Conclusion::Pending => Conclusion::Pending,
                    // sending the inquiry command has finished
                    Conclusion::Ready(_) => {
                        // change the internal state
                        *self.as_mut().state() = InquiryState::Initiated;
                        // and register this thinkable to be waken when the corresponding
                        // events will be received
                        let waker = cx.waker().clone();
                        let mut hci = self.as_mut().hci().lock();

                    }
                }
/*

                let waker = cx.waker().clone();
                let packet = this.packet.take().unwrap();
                this.transport.take_for(|trans| trans.send_waking_packet(packet, waker));
                this.state = InquiryState::Initiated;
                Conclusion::Pending
*/                
            },
            InquiryState::Initiated => {
                /*
                // the inquiry command h
                // we have initiated the inquiry so wait for the command completion, this is than
                // followed by the actual inquiry result events...
                let response = this.transport.take_for(|trans| trans.recv_response(HciPacketType::Event));
                if let Some(response) = response {
                    // inquiry successfully started, so we are pending and need to register our waker
                    // to be waken once the results are received
                    let waker = cx.waker().clone();
                    this.transport.take_for(|trans| {
                        // got awaken by either inquiry result or completion event
                        trans.set_event_waker(HciEventType::InquiryResult, waker.clone());
                        trans.set_event_waker(HciEventType::InquiryComplete, waker);
                    });
                    this.state = InquiryState::Running;
                    Conclusion::Pending
                } else {
                    // no successful inquiry start - we are done but with an empty list....
                    Conclusion::Ready(this.devices.take().unwrap())
                }
                */
            },
            InquiryState::Running => {
                /*
                // running state means we are waiting for inquiry results. Those could be up to a 
                // define maximum number of responses or a "time-out"
                // so once we got waken here check for the packet we have received
                let response = this.transport.take_for(|trans| trans.recv_response(HciPacketType::Event));
                if let Some(response) = response {
                    match HciEventInquiryResponse::try_from(response) {
                        Ok(result) => {
                            //info!("Inquiry Result {:#X?}", result);
                            let mut devices = this.devices.take().unwrap();
                            devices.extend(result.data);
                            this.devices.replace(devices);
                        },
                        Err(data) => match HciEventInquiryComplete::try_from(data) {
                            Ok(complete) => {
                                //info!("Inquiry Complete {:#X?}", complete);
                                return Conclusion::Ready(this.devices.take().unwrap());
                            },
                            Err(_) => error!("Unexpected inquiry response"),
                        },
                    };
                    let waker = cx.waker().clone();
                    this.transport.take_for(|trans| {
                        // got awaken by either inquiry result or completion event
                        trans.set_event_waker(HciEventType::InquiryResult, waker.clone());
                        trans.set_event_waker(HciEventType::InquiryComplete, waker.clone());
                    });
                    Conclusion::Pending
                } else {
                    // we got waken but there is no response... are we done ?
                    Conclusion::Ready(this.devices.take().unwrap())
                }
                */
            },
            InquiryState::Done => Conclusion::Ready(self.as_mut().devices().take().unwrap()),
        }
    }
}