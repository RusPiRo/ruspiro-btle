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
use crate::hci::events::*;
use crate::hctl::HcTransportLayer;
use crate::pin::Pin;
use crate::{warn, error};

const INQUIRY_LAP_LIAC: [u8; 3] = [0x00, 0x8B, 0x9E];   // Limited dedicated Inquiry Access Code
const INQUIRY_LAP_GIAC: [u8; 3] = [0x33, 0x8B, 0x9E];	// General unlimited Inquiry Access Code

pub struct InquireDevicesThinkable<T>
where T: HcTransportLayer + 'static
{
    hci: Arc<DataLock<Hci<T>>>,
    state: InquiryState,
    command: SendCommandThinkable<HciCommandInquiry, T>,
    devices: Option<Vec<HciEventInquiryResponseData>>,
}

#[derive(Copy, Clone, Debug)]
enum InquiryState {
    Initial,
    //Initiated,
    Running,
    Done,
}

impl<T> InquireDevicesThinkable<T>
where T: HcTransportLayer,
{
    unsafe_unpinned!(hci: Arc<DataLock<Hci<T>>>);
    unsafe_unpinned!(state: InquiryState);
    unsafe_unpinned!(command: SendCommandThinkable<HciCommandInquiry, T>);
    unsafe_unpinned!(devices: Option<Vec<HciEventInquiryResponseData>>);

    pub fn new(hci: Arc<DataLock<Hci<T>>>, length: InquiryLength) -> Self {
        Self {
            hci: hci.clone(),
            state: InquiryState::Initial,
            command: SendCommandThinkable::new(
                HciCommandInquiry::new(INQUIRY_LAP_GIAC, length, 5),
                hci,
            ),
            devices: Some(Vec::<HciEventInquiryResponseData>::new()),
        }
    }
}

impl<T> Thinkable for InquireDevicesThinkable<T>
where T: HcTransportLayer,
{
    type Output = Result<Vec<HciEventInquiryResponseData>, BoxError>;

    fn think(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        // let this = unsafe { self.get_unchecked_mut() };

        match self.state {
            InquiryState::Initial => {
                // initial state means we need to send the InquiryCommand
                let mut pin_command = Box::pin(self.as_mut().command());
                match pin_command.as_mut().think(cx) {
                    // sending the inquiry command is pending
                    Conclusion::Pending => Conclusion::Pending,
                    // sending the inquiry command has finished
                    Conclusion::Ready(_) => {
                        // change the internal state
                        *self.as_mut().state() = InquiryState::Running;
                        // and register this thinkable to be waken when the corresponding
                        // events will be received
                        let waker = cx.waker().clone();
                        let mut hci = self.as_mut().hci().lock();
                        hci.event_notify.insert(events::HciEventType::InquiryResult, (waker.clone(), None));
                        hci.event_notify.insert(events::HciEventType::InquiryComplete, (waker.clone(), None));
                        Conclusion::Pending
                    }
                }
            },
            InquiryState::Running => {
                // getting here means the inquiry command has been successfully processed by the host
                // and we received either an InquiryResult or an InquiryComplete
                let hci = self.as_ref().hci.clone();
                let mut hci = hci.lock();
                //let mut devices_option = self.as_mut().devices();
                if let Some(complete) = hci.event_notify.get(&events::HciEventType::InquiryComplete) {
                    if complete.1.is_some() {
                        // if the inquiry has finished, then remove the corresponding events from
                        // being waken
                        hci.event_notify.remove(&events::HciEventType::InquiryComplete);
                        hci.event_notify.remove(&events::HciEventType::InquiryResult);
                        // conclude with the device list
                        return Conclusion::Ready(
                            Ok(
                                self.as_mut().devices().take().unwrap()
                            )
                        );
                    }
                }

                if let Some(result) = hci.event_notify.get_mut(&events::HciEventType::InquiryResult) {
                    if let Some(result) = result.1.take() {
                        // we retrieved some devices, add them to the device list
                        if let Ok(new_devices) = events::HciEventInquiryResponse::try_from(result) {
                            let mut devices = self.as_mut().devices().take().unwrap();
                            devices.extend(new_devices.data);
                            self.as_mut().devices().replace(devices);
                            return Conclusion::Pending
                        } else {
                            error!("not an inquiry response...");
                        }
                    }
                }
                // if we got here we where waken with neither a result nor a completion, so keep
                // waiting ?
                warn!("inquiry woken for whatever reason...");
                Conclusion::Pending
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
            InquiryState::Done => Conclusion::Ready(
                Ok(
                    self.as_mut().devices().take().unwrap())
                ),
        }
    }
}