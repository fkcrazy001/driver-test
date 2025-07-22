use core::ptr::NonNull;

use dma_api::{DVec, Direction};
use log::error;
use tock_registers::register_bitfields;

use crate::rxtx::decs::Descriptor;
mod decs;
pub mod rx;
pub mod tx;

pub struct Ring<D: Descriptor> {
    // base va of this ring
    base_va: NonNull<u8>,
    desc_table: DVec<D>,
    tail_reg: usize,
    head_reg: usize,
    mirror_tail: u32,
    mirror_head: u32,
}

impl<D: Descriptor> Ring<D> {
    pub fn new(base_va: NonNull<u8>, desc_n: usize, tail_reg: usize, head_reg: usize) -> Self {
        let desc_table =
            DVec::zeros(desc_n, DESC_TABLE_ALLIGN_MIN, Direction::Bidirectional).unwrap();
        Self {
            base_va,
            desc_table,
            tail_reg,
            head_reg,
            mirror_head: 0,
            mirror_tail: 0,
        }
    }
    pub fn init_tail_head(&mut self) {
        self.write_reg(self.tail_reg, 0u32);
        self.write_reg(self.head_reg, 0u32);
        self.mirror_head = 0;
        self.mirror_tail = 0;
    }
    pub fn write_reg<T>(&mut self, offset: usize, data: T) {
        unsafe { self.base_va.add(offset).cast().write_volatile(data) }
    }
    pub fn read_reg<T>(&self, offset: usize) -> T {
        unsafe { self.base_va.add(offset).cast().read_volatile() }
    }

    pub fn desc_table_base(&self) -> u64 {
        self.desc_table.bus_addr()
    }
    pub fn desc_table_size(&self) -> u32 {
        (self.desc_table.len() * core::mem::size_of::<D>()) as u32
    }
    pub fn get_available(&mut self) -> Option<(D, usize)> {
        let head: u32 = self.get_head();
        if head == self.mirror_head {
            return None;
        }
        let res = self.desc_table.get(self.mirror_head as usize).unwrap();
        let head = self.mirror_head;
        self.mirror_head = (self.mirror_head + 1) % self.desc_table.len() as u32;
        Some((res, head as usize))
    }
    pub fn add_desc(&mut self, desc: D) -> Result<usize, ()> {
        let head = self.mirror_head;
        let tail: u32 = self.get_tail();
        let n_tail = (tail + 1) % self.desc_table.len() as u32;
        if n_tail == head {
            error!("ring full!");
            return Err(());
        }
        self.desc_table.set(tail as usize, desc);
        self.write_reg(self.tail_reg, n_tail);
        Ok(tail as usize)
    }
    pub fn get_tail(&self) -> u32 {
        self.read_reg(self.tail_reg)
    }
    fn get_head(&self) -> u32 {
        self.read_reg(self.head_reg)
    }
}

const DESC_TABLE_ALLIGN_MIN: usize = 128;

const RDBAL: usize = 0xC000; // RX Descriptor Base Address Low
const RDBAH: usize = 0xC004; // RX Descriptor Base Address High
const RDLEN: usize = 0xC008; // RX Descriptor Length
const SRRCTL: usize = 0xC00C; // RX Descriptor Control
const RDH: usize = 0xC010; // RX Descriptor Head
const RDT: usize = 0xC018; // RX Descriptor Tail
const RXDCTL: usize = 0xC028; // RX Descriptor Control
// const RXCTL: usize = 0xC014; // RX Control
// const RQDPC: usize = 0xC030; // RX Descriptor Polling Control

// TX descriptor registers
const TDBAL: usize = 0xE000; // TX Descriptor Base Address Low
const TDBAH: usize = 0xE004; // TX Descriptor Base Address High
const TDLEN: usize = 0xE008; // TX Descriptor Length
const TDH: usize = 0xE010; // TX Descriptor Head
const TDT: usize = 0xE018; // TX Descriptor Tail
const TXDCTL: usize = 0xE028; // TX Descriptor Control
// const TDWBAL: usize = 0xE038; // TX Descriptor Write Back Address Low
// const TDWBAH: usize = 0xE03C; // TX Descriptor Write Back Address High

register_bitfields! [
    // First parameter is the register width. Can be u8, u16, u32, or u64.
    u32,

    RDLEN [
        LEN OFFSET(7) NUMBITS(13)[],
    ],

    pub SRRCTL [
        BSIZEPACKET OFFSET(0) NUMBITS(7)[],
        BSIZEHEADER OFFSET(8) NUMBITS(4)[],
        RDMTS OFFSET(20) NUMBITS(5)[],
        DESCTYPE OFFSET(25) NUMBITS(3)[
            Legacy = 0b000,
            AdvancedOneBuffer = 0b001,
            AdvancedHeaderSplitting = 0b010,
            AdvancedHeaderReplicationAlways = 0b011,
            AdvancedHeaderReplicationLargePacket = 0b100,
        ],
        SECRC OFFSET(26) NUMBITS(1)[
            DoNotStrip = 0,
            Strip = 1,
        ],
        DROP_EN OFFSET(31) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
    ],

    pub RXDCTL [
        PTHRESH OFFSET(0) NUMBITS(5)[],
        HTHRESH OFFSET(8) NUMBITS(5)[],
        WTHRESH OFFSET(16) NUMBITS(5)[],
        ENABLE OFFSET(25) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        SWFLUSH OFFSET(26) NUMBITS(1)[],
    ],

    pub TXDCTL [
        PTHRESH OFFSET(0) NUMBITS(5)[],
        HTHRESH OFFSET(8) NUMBITS(5)[],
        WTHRESH OFFSET(16) NUMBITS(5)[],
        ENABLE OFFSET(25) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        SWFLUSH OFFSET(26) NUMBITS(1)[],
    ],


];
