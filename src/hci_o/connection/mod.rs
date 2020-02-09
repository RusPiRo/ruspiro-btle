/*************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Handle Connections
//! 

use super::*;

pub struct HandleInboundConnectionsThought {
    transport: SharedTransportLayer,
}

impl HandleInboundConnectionsThought {
    pub fn new(transport: SharedTransportLayer) -> Self {
        Self {
            transport,
        }
    }
}

impl Thinkable for HandleInboundConnectionsThought {
    type Output = ();

    /// Thinking "forever" to handle incomming connection requests
    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        // when thinking on this one check for a connection request beeing available
        if let Some(response) = self.transport.take_for(|trans| trans.recv_response(HciPacketType::Event)) {
            match HciEventConnectionRequest::try_from(response) {
                Ok(conn_request) => info!("connection request {:?}", conn_request),
                _ => (),
            }
        }
        // register myself to be awaken as soon as a connection request event will be 
        // received
        let waker = cx.waker().clone();
        self.transport.take_for(|trans| trans.set_event_waker(HciEventType::ConnectionRequest, waker));
        
        Conclusion::Pending
    }
}