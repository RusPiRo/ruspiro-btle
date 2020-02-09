/*************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Handle Connections
//! 

use super::*;
use crate::hci::events::{
    HciEventConnectionRequest,
    HciEventConnectionComplete,
};
use crate::hctl::HcTransportLayer;
use crate::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum HciConnectionLinkType {
    /// Synchronous Connection Oriented
    Sco = 0x00,
    /// Asynchronous Connection Less
    Acl = 0x01,
    Unknown,
}

#[derive(Debug)]
#[repr(u8)]
pub enum HciConnectionRole {
    Master = 0x00,
    Slave = 0x01,
    Unknown,
}

impl From<u8> for HciConnectionLinkType {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => HciConnectionLinkType::Sco,
            0x01 => HciConnectionLinkType::Acl,
            _ => HciConnectionLinkType::Unknown,
        }
    }
}

#[derive(Debug)]
#[repr(u8)]
pub enum HciEncryptionType {
    Disabled = 0x00,
    Point2Point = 0x01,
    P2PBroadcast = 0x02,
    Unknown,
}

impl From<u8> for HciEncryptionType {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => HciEncryptionType::Disabled,
            0x01 => HciEncryptionType::Point2Point,
            0x02 => HciEncryptionType::P2PBroadcast,
            _ => HciEncryptionType::Unknown,
        }
    }
}

pub struct HandleInboundConnectionsThinkable<T>
where T: HcTransportLayer + 'static {
    hci: Arc<DataLock<Hci<T>>>,
    first: AtomicBool,
}

impl<T> HandleInboundConnectionsThinkable<T>
where T: HcTransportLayer
{
    unsafe_unpinned!(hci: Arc<DataLock<Hci<T>>>);

    pub fn new(hci: Arc<DataLock<Hci<T>>>) -> Self {
        Self {
            hci,
            first: AtomicBool::new(true),
        }
    }
}

impl<T> Thinkable for HandleInboundConnectionsThinkable<T>
where T: HcTransportLayer
{
    type Output = ();

    /// Thinking "forever" to handle incomming connection requests
    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        // when thinking on this one check for a connection request beeing available
        let hci = self.as_ref().hci.clone();
        let mut hci = hci.lock();
        if let Some(request) = hci.event_notify.get_mut(&HciEventType::ConnectionRequest) {
            if let Some(request) = request.1.take() {
                info!("Connection request from: {:#X?}", request);
                match HciEventConnectionRequest::try_from(request) {
                    Ok(request) => {
                        info!("Connection request from: {:#X?}", request);
                        // just spawn the connection accepted command for any incomming device?
                        // TODO: implement a filter to not accept any arbitrary device to connect
                        spawn(
                            Hci::send_command(
                                self.hci.clone(),
                                commands::HciCommandAcceptConnection::new(
                                    request.address(),
                                    HciConnectionRole::Slave,
                                )
                            ).map(|_| info!("connection accepted sent"))
                        );
                    },
                    Err(data) => warn!("wrong event received in connection thinkable {:#X?}", data),
                }
            }
        }

        if let Some(complete) = hci.event_notify.get_mut(&HciEventType::ConnectionComplete) {
            if let Some(complete) = complete.1.take() {
                match HciEventConnectionComplete::try_from(complete) {
                    Ok(complete) => info!("Connection complete: {:#X?}", complete),
                    Err(data) => warn!("wrong event received in connection thinkable {:#?}", data),
                }
            }
        }

        if self.first.load(Ordering::Relaxed) {
            self.first.store(false, Ordering::Release);
            // if thinking here the first time register ourself to get woken as soon as a connection
            // request arises. This registration stays for ever...
            let waker = cx.waker().clone();
            hci.event_notify.insert(HciEventType::ConnectionRequest, (waker.clone(), None));
            hci.event_notify.insert(HciEventType::ConnectionComplete, (waker, None));
        }
        
        Conclusion::Pending
    }
}