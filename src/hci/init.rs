use crate::pin::Pin;
use crate::pin_utils::*;
use super::{*, commands::*, events::*, firmware::*};

pub enum HciInitThinkable<T>
where T: HcTransportLayer + 'static,
{
    /// Initial state, never thought of
    State_0 {context: InitContext<T>},
    /// send the reset command and wai for it
    State_1 {context: InitContext<T>, wait_for: Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>},
    /// send the download minidriver command and wait for it
    State_2 {context: InitContext<T>, wait_for: Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>},
    /// wait for the minidriver upload to complete
    State_3 {context: InitContext<T>, wait_for: Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>},
    /// wait for restart of the BT host
    State_4 {context: InitContext<T>, wait_for: Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>},
    /*
    State_0(Arc<DataLock<Hci<T>>>, Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>),
    //State_0(SendCommandThinkable<HciCommandReset, T>),
    /// Next state - reset done, request FW upload
    State_1(Arc<DataLock<Hci<T>>>, Result<(), BoxError>, Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>),
    /// Next state - upload FW in progress
    State_2(Arc<DataLock<Hci<T>>>, Result<(), BoxError>, Pin<Box<dyn Thinkable<Output = Result<(), BoxError>>>>),
    /// Next state - waiting for device reset
    State_3(Arc<DataLock<Hci<T>>>, Result<(), BoxError>, Pin<Box<dyn Thinkable<Output = ()>>>),
    /// Last state - final processing and concluding on this Thinkable
    State_4(Arc<DataLock<Hci<T>>>),
    */
    /// Empty state to ensure previous stored states are properly dropped at state transition
    Empty,
}

struct InitContext<T> {
    hci: Arc<DataLock<Hci<T>>>,
}

unsafe impl<T> Send for HciInitThinkable<T>
where T: HcTransportLayer
{}

impl<T> HciInitThinkable<T>
where T: HcTransportLayer,
{
    pub fn new(hci: Arc<DataLock<Hci<T>>>) -> Self {
        Self::State_0(
            hci.clone(),
            Box::pin(
                SendCommandThinkable::new(
                    HciCommandReset::new(),
                    hci,
                )
            )
        )
    }
}

impl<T> Thinkable for HciInitThinkable<T>
where T: HcTransportLayer,
{
    type Output = ();

    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        // this loop allows to switch the wait states immediately in case of any
        // immediate conclusion of a thinkable
        loop {
            // get the next state based on the current one
            let next_state = match this {
                State_0 { context } => {
                    // the initial state just requests the reset of the BT host using the
                    // respective command that need to be awaited for
                    let hci = context.hci.clone();
                    Self::State_1 {
                        context,
                        wait_for: Box::pin(
                            send_command(
                                HciCommandReset::new(),
                                hci,
                            )
                        )
                    }
                },
                State_1 { context, wait_for} => {
                    // think on the reset command until it has returned a conlcusion
                    match wait_for.as_mut().think(cx) {
                        // short cut in case we still need to think on it...
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(_) => {
                            // we can move to the next state
                            let hci = context.hci.clone();
                            Self::State_2 {
                                context,
                                wait_for: Box::pin(
                                    send_command(
                                        HciCommandDownloadMiniDriver::new(),
                                        hci
                                    )
                                )
                            }
                        }
                    }
                },
                State_2 { context, wait_for} => {
                    // think on the minidriver downlod request command to the host until it has returned a conlcusion
                    match wait_for.as_mut().think(cx) {
                        // short cut in case we still need to think on it...
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(_) => {
                            // we can move to the next state
                            let hci = context.hci.clone();
                            Self::State_3 {
                                context,
                                wait_for: Box::pin(
                                    firmware::UploadFirmwareThinkable::new(hci)
                                )
                            }
                        }
                    }
                },
                State_3 { context, wait_for} => {
                    // think on the actual minidriver upload to the host until it has returned a conlcusion
                    match wait_for.as_mut().think(cx) {
                        // short cut in case we still need to think on it...
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(_) => {
                            // we can move to the next state
                            let hci = context.hci.clone();
                            Self::State_4 {
                                context,
                                wait_for: Box::pin(
                                    wait(Mseconds(500), ())
                                )
                            }
                        }
                    }
                },
                State_4 { context, wait_for} => {
                    // think on the waiting until it has returned a conlcusion
                    match wait_for.as_mut().think(cx) {
                        // short cut in case we still need to think on it...
                        Conclusion::Pending => return Conclusion::Pending,
                        // this has been the final state so we come to a conclusion here
                        Conclusion::Ready(_) => return Conclusion::Ready(()),
                    }
                },
            }

            // use the next state to update myself
            // to ensure proper destructor calling do this with an immediate step
            *this = Self::Empty;
            *this = next_state;

            /*
            // this loop covers the case where each underlying thinkable immediately returns a
            // conclusion as in this scenario no waker would be registered to wake this thinkable
            // again
            let next = match this {
                Self::State_0(hci, thinkable) => {
                    info!("think state_0");
                    match thinkable.as_mut().think(cx) {
                        // pending can be immediately returned, the underlying Thinkable has registered
                        // the waker to get woken
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(result) => {
                            let hci_clone = hci.clone();
                            Self::State_1(
                                hci_clone.clone(),
                                result,
                                Box::pin(
                                    SendCommandThinkable::new(
                                        HciCommandDownloadMiniDriver::new(),
                                        hci_clone,
                                    )
                                )
                            )
                        }
                    }
                },
                Self::State_1(hci, state_0_result, thinkable) => {
                    info!("think state_1");
                    match thinkable.as_mut().think(cx) {
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(result) => {
                            let hci_clone = hci.clone();
                            Self::State_2(
                                hci_clone.clone(),
                                result,
                                Box::pin(
                                    firmware::UploadFirmwareThinkable::new(hci_clone)
                                )
                            )
                        }
                    }
                },
                Self::State_2(hci, state_1_result, thinkable) => {
                    info!("think state_2");
                    match thinkable.as_mut().think(cx) {
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(result) => {
                            let hci_clone = hci.clone();
                            Self::State_3(
                                hci_clone.clone(),
                                result,
                                Box::pin(
                                    wait(Mseconds(500), ())
                                )
                            )
                        }
                    }
                },
                Self::State_3(hci, state_2_result, thinkable) => {
                    info!("think state_3");
                    match thinkable.as_mut().think(cx) {
                        Conclusion::Pending => return Conclusion::Pending,
                        Conclusion::Ready(_) => {
                            let hci_clone = hci.clone();
                            Self::State_4(
                                hci_clone
                            )
                        }
                    }
                },
                Self::State_4(hci) => {
                    info!("think state_4");
                    return Conclusion::Ready(());
                },
                _ => {
                    error!("unhandled state...");
                    unimplemented!();
                }
            };
            // switch the state
            *this = Self::Empty;
            *this = next;
            */
        }
    }
}