#![no_std]

use core::fmt::Display;

use alloc::{
    boxed::Box,
    string::String,
    vec::{self, Vec},
};
use crab_usb::{
    Class, Device, Direction, EndpointBulkIn, EndpointBulkOut, EndpointDescriptor, EndpointType,
    Interface, Recipient, Request, RequestType, err::USBError,
};
use dma_api::{DVec, Direction::Bidirectional, Direction::FromDevice, Direction::ToDevice};
use log::debug;
use usb_if::host::ControlSetup;

extern crate alloc;

pub struct Ch341 {
    usb_device: Device,
    in_ep: Option<EndpointBulkIn>,
    out_ep: Option<EndpointBulkOut>,
    max_in_pkt_size: usize,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Clone)]
pub enum Ch341Req {
    CH341_CMD_R = 0x95,  //十进制149
    CH341_CMD_W = 0x9A,  //十进制154
    CH341_CMD_C1 = 0xA1, //十进制161
    CH341_CMD_C2 = 0xA4, //十进制164
    CH341_CMD_C3 = 0x5F, //十进制95
}

impl Ch341 {
    pub fn new(d: Device) -> Option<Self> {
        let desc = &d.descriptor;
        debug!("device desc: {desc:#?}");
        if desc.product_id != 0x7523
            || desc.vendor_id != 0x1a86
            || !matches!(desc.class(), Class::Communication)
        {
            return None;
        }
        Some(Self {
            usb_device: d,
            in_ep: None,
            out_ep: None,
            max_in_pkt_size: 0,
        })
    }

    async fn ch341_control_out(
        interface: &mut Interface,
        req: Ch341Req,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<usize, USBError> {
        interface
            .control_out(
                ControlSetup {
                    request_type: RequestType::Vendor,
                    recipient: Recipient::Device,
                    request: Request::Other(req as u8),
                    value,
                    index,
                },
                data,
            )
            .await?
            .await
            .map_err(|e| USBError::TransferError(e))
    }
    async fn ch341_control_in(
        interface: &mut Interface,
        req: Ch341Req,
        value: u16,
        index: u16,
        data: &mut [u8],
    ) -> Result<usize, USBError> {
        interface
            .control_in(
                ControlSetup {
                    request_type: RequestType::Vendor,
                    recipient: Recipient::Device,
                    request: Request::Other(req as u8),
                    value,
                    index,
                },
                data,
            )?
            .await
            .map_err(|e| USBError::TransferError(e))
    }

    async fn ch341_interface_init(&mut self, interface: &mut Interface) -> Result<(), USBError> {
        // init
        let mut empty = [0; 0];
        let mut version = [0; 2];
        let read =
            Self::ch341_control_in(interface, Ch341Req::CH341_CMD_C3, 0, 0, &mut version).await?;
        debug!("read {read}, version {version:#?}");
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_C1, 0, 0, &mut empty).await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x1312, 0xd982, &mut empty)
            .await?;

        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x0f2c, 0x0007, &mut empty)
            .await?;
        Self::ch341_control_in(interface, Ch341Req::CH341_CMD_R, 0x2518, 0, &mut version).await?;
        Self::ch341_control_in(interface, Ch341Req::CH341_CMD_R, 0, 0x0706, &mut version).await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x2727, 0, &mut empty).await?;

        Self::ch341_control_out(
            interface,
            Ch341Req::CH341_CMD_C1,
            0xc39c,
            0xb282,
            &mut empty,
        )
        .await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x0f2c, 0x0008, &mut empty)
            .await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x2727, 0x0000, &mut empty)
            .await?;

        // now config bd rate
        Self::ch341_control_out(
            interface,
            Ch341Req::CH341_CMD_C1,
            0x2727,
            0xb282,
            &mut empty,
        )
        .await?;

        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x0f2c, 0x8, &mut empty).await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x2727, 0, &mut empty).await?;
        Self::ch341_control_out(
            interface,
            Ch341Req::CH341_CMD_C1,
            0xc39c,
            0xcc83,
            &mut empty,
        )
        .await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x0f2c, 7, &mut empty).await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_C2, 0x00df, 0, &mut empty).await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_W, 0x2727, 0, &mut empty).await?;
        Self::ch341_control_out(interface, Ch341Req::CH341_CMD_C2, 0x009f, 0, &mut empty).await?;

        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), USBError> {
        let device = &mut self.usb_device;
        for config in device.configurations().iter() {
            debug!("Configuration: {config:?}");
        }
        let config = &device.configurations()[0];
        debug!("use configuration 0");
        let interface = config
            .interfaces
            .iter()
            .find(|interface| {
                let aif = interface.first_alt_setting();
                matches!(aif.class(), Class::Communication)
            })
            .ok_or(USBError::NotFound)?
            .first_alt_setting();
        debug!("Using interface: {interface:?}");
        let mut interface = device
            .claim_interface(interface.interface_number, interface.alternate_setting)
            .await?;

        // below code mainly copies from linux and https://github.com/arceos-usb/arceos_experiment, branch usb-camera-base,
        //  bear with me, cause nor do I know what these mean

        for ep in interface.descriptor.endpoints.clone() {
            match (ep.transfer_type, ep.direction) {
                (EndpointType::Bulk, Direction::In) => {
                    self.in_ep = Some(interface.endpoint_bulk_in(ep.address)?);
                    self.max_in_pkt_size = ep.max_packet_size as usize;
                }
                (EndpointType::Bulk, Direction::Out) => {
                    self.out_ep = Some(interface.endpoint_bulk_out(ep.address)?)
                }
                _ => debug!("Ignoring endpoint: {ep:?}"),
            }
        }

        if self.in_ep.is_none() || self.out_ep.is_none() {
            return Err(USBError::NotFound);
        }
        self.ch341_interface_init(&mut interface).await?;
        Ok(())
    }
    pub async fn recv(&mut self) -> Result<Vec<u8>, USBError> {
        debug!("try to read some data");
        if let Some(ep) = self.in_ep.as_mut() {
            let mut data = [0u8; 64];
            let n = ep.submit(&mut data)?.await?;
            Ok(data[..n].to_vec())
        } else {
            Err(USBError::NotInitialized)
        }
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<usize, USBError> {
        debug!("try to send some data");
        if let Some(ep) = self.out_ep.as_mut() {
            let n = ep.submit(data)?.await?;
            Ok(n)
        } else {
            Err(USBError::NotInitialized)
        }
    }
}
