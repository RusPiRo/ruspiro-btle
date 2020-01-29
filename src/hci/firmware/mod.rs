/*************************************************************************************************** 
 * Copyright (c) 2019 by the authors
 * 
 * Author: André Borrmann 
 * License: Apache License 2.0
 **************************************************************************************************/

//! # Firmware related Thoughts
//! 

use super::*;

// TODO: check for alignment requirements on this external data
static FIRMWARE: &'static [u8] = include_bytes!("./BCM4345C0.hcd"); //PI 3 B+
//static FIRMWARE: &'static [u8] = include_bytes!("./BCM43430A1.hcd"); //PI 2/3


pub struct UploadFWThought {
    fw_offset: usize,
    packet: Option<HciPacket<HciCommandVendorBcm>>,
    transport: SharedTransportLayer,
}

impl UploadFWThought {

    pub fn new(transport: SharedTransportLayer) -> Self {
        let mut thought = Self {
            fw_offset: 0,
            packet: None,
            transport
        };
        let packet = thought.next_packet();
        thought.packet.replace(packet);

        thought
    }

    // get the next packet to upload the fw blob to the BT Host
    fn next_packet(&mut self) -> HciPacket<HciCommandVendorBcm> {
        let op_code: HciCommand = (FIRMWARE[self.fw_offset] as u16 | (FIRMWARE[self.fw_offset+1] as u16) << 8).into();
        let chunk_size = FIRMWARE[self.fw_offset+2];
        let chunk_start = self.fw_offset+3;
        let chunk_end = chunk_start + chunk_size as usize;
        // update the offset to point to the next chunk in the FW blob
        self.fw_offset += (chunk_size + 3) as usize;

        HciPacket {
            p_type: HciPacketType::Command,
            p_data: HciCommandVendorBcm::new(
                op_code,
                &FIRMWARE[chunk_start..chunk_end],
            ),
        }
    }
}

impl Thinkable for UploadFWThought {
    type Output = ();

    fn think(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Conclusion<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.packet.is_some() {
            //info!("send fw chunk");
            let waker = cx.waker().clone();
            let packet = this.packet.take().unwrap();
            this.transport.take_for(|trans| trans.send_waking_packet(packet, waker));
            Conclusion::Pending
        } else {
            let response = this.transport.take_for(|trans| trans.recv_response());
            if let Some(response) = response {
                //info!("upload response {:?}", response);
                if this.fw_offset < FIRMWARE.len() {
                    let next = this.next_packet();
                    this.packet.replace(next);
                    cx.waker().wake_by_ref();
                    return Conclusion::Pending;
                } else {
                    //info!("fw upload done");
                    return Conclusion::Ready(());
                }
            }
            Conclusion::Pending
        }
    }
}