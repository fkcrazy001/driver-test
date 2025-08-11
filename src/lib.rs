#![no_std]

use alloc::{boxed::Box, vec};
use crab_usb::{
    Class, Device, Direction, EndpointDescriptor, EndpointType, Interface, Recipient, Request,
    RequestType, err::USBError,
};
use dma_api::{DVec, Direction::Bidirectional, Direction::FromDevice, Direction::ToDevice};
use log::debug;
use usb_if::host::ControlSetup;

extern crate alloc;

pub struct Ch341 {
    usb_device: Device,
    ctrl_buf: DVec<u8>,
    status_buf: DVec<u8>,
    in_ep: Option<EndpointDescriptor>,
    out_ep: Option<EndpointDescriptor>,
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
            ctrl_buf: DVec::zeros(2, 2, Bidirectional).unwrap(),
            status_buf: DVec::zeros(2, 2, FromDevice).unwrap(),
            in_ep: None,
            out_ep: None,
        })
    }

    async fn ch341_control_out(
        interface: &mut Interface,
        req: u8,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<usize, USBError> {
        interface
            .control_out(
                ControlSetup {
                    request_type: RequestType::Vendor,
                    recipient: Recipient::Device,
                    request: Request::Other(req),
                    value,
                    index,
                },
                data,
            )
            .await?
            .await
            .map_err(|e| USBError::Other(Box::new(e)))
    }
    async fn ch341_control_in(
        interface: &mut Interface,
        req: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
    ) -> Result<usize, USBError> {
        interface
            .control_in(
                ControlSetup {
                    request_type: RequestType::Vendor,
                    recipient: Recipient::Device,
                    request: Request::Other(req),
                    value,
                    index,
                },
                data,
            )?
            .await
            .map_err(|e| USBError::Other(Box::new(e)))
    }

    async fn ch341_interface_init(&mut self, interface: &mut Interface) -> Result<(), USBError> {
        // init
        let mut empty = [0; 0];
        let mut version = [0; 2];
        let read = Self::ch341_control_in(interface, 0x5f, 0, 0, &mut version).await?;
        debug!("read {read}, version {version:#?}");
        Self::ch341_control_out(interface, 0xa1, 0, 0, &mut empty).await?;
        Self::ch341_control_out(interface, 0x9a, 0x1312, 0xd982, &mut empty).await?;

        Self::ch341_control_out(interface, 0x9a, 0x0f2c, 0x0007, &mut empty).await?;
        Self::ch341_control_in(interface, 0x95, 0x2518, 0, &mut version).await?;
        Self::ch341_control_in(interface, 0x95, 0, 0x0706, &mut version).await?;
        Self::ch341_control_out(interface, 0x9a, 0x2727, 0, &mut empty).await?;

        Self::ch341_control_out(interface, 0xa1, 0xc39c, 0xb282, &mut empty).await?;
        Self::ch341_control_out(interface, 0x9a, 0x0f2c, 0x0008, &mut empty).await?;
        Self::ch341_control_out(interface, 0x9a, 0x2727, 0x0000, &mut empty).await?;

        Ok(())
    }

    pub async fn init(&mut self) -> Result<(), USBError> {
        let device = &mut self.usb_device;
        for config in device.configurations.iter() {
            debug!("Configuration: {config:?}");
        }
        let config = &device.configurations[0];
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

        for ep in &interface.descriptor.endpoints {
            match (ep.transfer_type, ep.direction) {
                (EndpointType::Bulk, Direction::In) => self.in_ep = Some(ep.clone()),
                (EndpointType::Bulk, Direction::Out) => self.out_ep = Some(ep.clone()),
                _ => debug!("Ignoring endpoint: {ep:?}"),
            }
        }

        if self.in_ep.is_none() || self.out_ep.is_none() {
            return Err(USBError::NotFound);
        }
        self.ch341_interface_init(&mut interface).await?;
        Ok(())
    }
}
