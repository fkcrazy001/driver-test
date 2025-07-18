#![no_std]

use core::{cell::RefCell, ptr::NonNull};

extern crate alloc;

mod mac;
#[macro_use]
pub mod misc;
mod phy;

pub use trait_ffi::impl_extern_trait;

pub use crate::mac::MacStatus;

pub struct Igb {
    mac: RefCell<mac::Mac>,
    phy: phy::Phy,
}

impl Igb {
    pub fn new(base: NonNull<u8>) -> Self {
        let mac = RefCell::new(mac::Mac::new(base));
        Self {
            phy: phy::Phy::new(1, mac.clone()),
            mac,
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

        // get fc in phy and set fc in mac
        // force set full duplex
        Ok(())
    }
    pub fn status(&mut self) -> MacStatus {
        self.mac.borrow().status()
    }
}

#[derive(Debug, Clone)]
pub enum Speed {
    Mb10,
    Mb100,
    Mb1000,
}
