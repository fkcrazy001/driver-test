#![no_std]

use core::ops::Deref;

use core::{cell::RefCell, ptr::NonNull};

extern crate alloc;

mod mac;
#[macro_use]
pub mod misc;
mod phy;

mod rxtx;

use alloc::vec::Vec;
use dma_api::{DVec, Direction};
use log::debug;
pub use trait_ffi::impl_extern_trait;

use crate::mac::MacAddr;
pub use crate::mac::MacStatus;
use crate::rxtx::{rx::RxRing, tx::TxRing};

pub struct Pkt {
    buff: DVec<u8>,
    /// for debug use
    dir: Direction,
}

impl Pkt {
    fn new(buff: Vec<u8>, dir: Direction) -> Self {
        let buff = DVec::from_vec(buff, dir);
        Self { buff, dir }
    }
    pub fn new_rx(buff: Vec<u8>) -> Self {
        Self::new(buff, Direction::FromDevice)
    }

    pub fn new_tx(buff: Vec<u8>) -> Self {
        Self::new(buff, Direction::ToDevice)
    }

    pub fn bus_addr(&self) -> u64 {
        self.buff.bus_addr()
    }
}

impl Deref for Pkt {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.buff.as_ref()
    }
}

impl Drop for Pkt {
    fn drop(&mut self) {
        debug!(
            "Pkt @{:x} drop, len = {}, dir = {:?}",
            self.bus_addr(),
            self.buff.len(),
            self.dir
        );
    }
}

pub struct Igb {
    mac: RefCell<mac::Mac>,
    phy: phy::Phy,
    queue: Vec<Option<(rxtx::rx::RxRing, rxtx::tx::TxRing)>>,
}

impl Igb {
    pub fn new(base: NonNull<u8>) -> Self {
        let mac = RefCell::new(mac::Mac::new(base));
        let mut q = Vec::with_capacity(16);
        for _ in 0..16 {
            q.push(None);
        }
        Self {
            phy: phy::Phy::new(1, mac.clone()),
            mac,
            queue: q,
        }
    }
    pub fn check_vid_did(vid: u16, did: u16) -> bool {
        vid == 0x8086 && [0x10C9, 0x1533].contains(&did)
    }
    #[allow(clippy::result_unit_err)]
    pub fn open(&mut self) -> Result<(), ()> {
        // disable and reset mac
        self.mac.borrow_mut().disable_irq();
        self.mac.borrow_mut().reset()?;
        self.mac.borrow_mut().disable_irq();

        // set mac link up

        // phy initialization
        // • reset phy, done in mac reset
        // • Setting preferred link configuration for advertisement during the auto-negotiation process
        // • Restarting the auto-negotiation process
        // • Reading auto-negotiation status from the PHY
        // • Forcing the PHY to a specific link configuration
        self.phy.power_up()?;
        self.phy.enable_auto_negotiation()?;

        // link up mac
        self.mac.borrow_mut().link_up();

        self.phy.wait_for_negotiate()?;

        // @todo: get fc in phy and set fc in mac
        // @todo: clear statics
        // enable rx and tx
        self.mac.borrow_mut().enable_rx_tx();
        Ok(())
    }
    pub fn status(&mut self) -> MacStatus {
        self.mac.borrow().status()
    }
    #[allow(clippy::result_unit_err)]
    pub fn alloc_new_qeueu(&mut self, qidx: usize, desc_n: usize, pkt_size: u32) -> Result<(), ()> {
        if self.queue[qidx].is_some() {
            return Ok(());
        }
        let va = unsafe { self.mac.borrow().base_addr().add(0x40 * qidx) };
        let rx_ring = RxRing::new(va, desc_n, pkt_size);
        let tx_ring = TxRing::new(va, desc_n);
        self.queue[qidx] = Some((rx_ring, tx_ring));
        Ok(())
    }
    pub fn receive(&mut self, qidx: usize) -> Option<Pkt> {
        let rxq = &mut self.queue[qidx].as_mut().unwrap().0;
        rxq.receive()
    }
    #[allow(clippy::result_unit_err)]
    pub fn transmit(&mut self, qidx: usize, p: Pkt) -> Result<usize, ()> {
        let txq = &mut self.queue[qidx].as_mut().unwrap().1;
        txq.transmit(p)
    }

    pub fn mac_addr(&self) -> MacAddr {
        self.mac.borrow().mac_addr()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Speed {
    Mb10,
    Mb100,
    Mb1000,
}
