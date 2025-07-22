use core::ptr::NonNull;

use alloc::vec::Vec;
use log::debug;
use tock_registers::register_bitfields;

use crate::{
    Pkt,
    rxtx::{Ring, TDBAH, TDBAL, TDH, TDLEN, TDT, TXDCTL, decs::Descriptor},
};

pub struct TxRing {
    base: Ring<TxDesc>,
    meta_ls: Vec<Option<Pkt>>,
}

impl Drop for TxRing {
    fn drop(&mut self) {
        debug!("tx ring droped!!");
        // 禁用队列
        self.base.write_reg(TXDCTL, (TXDCTL::ENABLE::CLEAR).value);
    }
}

impl TxRing {
    pub fn new(va: NonNull<u8>, desc_n: usize) -> Self {
        // Program the TCTL register according to the MAC behavior needed.
        // If work in half duplex mode is expected, program the TCTL_EXT.COLD field. For internal PHY mode the
        // default value of 0x41 is OK. For SGMII mode, a value reflecting the 82576 and the PHY SGMII delays
        // should be used. A suggested value for a typical PHY is 0x46 for 10 Mbps and 0x4C for 100 Mbps.
        // The following should be done once per transmit queue:
        // • Allocate a region of memory for the transmit descriptor list.
        // • Program the descriptor base address with the address of the region.
        // • Set the length register to the size of the descriptor ring.
        // • Program the TXDCTL register with the desired TX descriptor write back policy. Suggested values
        // are:
        // — WTHRESH = 1b
        // — All other fields 0b.
        // • If needed, set the TDWBAL/TWDBAH to enable head write back
        // • Enable the queue using TXDCTL.ENABLE (queue zero is enabled by default).
        // • Poll the TXDCTL register until the ENABLE bit is set.
        // Note: The tail register of the queue (TDT[n]) should not be bumped until the queue is enabled.
        // Enable transmit path by setting TCTL.EN. This should be done only after all other settings are done.

        let mut base = Ring::new(va, desc_n, TDT, TDH);
        let desc_table_base = base.desc_table_base();
        base.write_reg(TXDCTL, TXDCTL::ENABLE::CLEAR.value);

        base.write_reg(TDBAL, desc_table_base as u32);
        base.write_reg(TDBAH, (desc_table_base >> 32) as u32);
        base.write_reg(TDLEN, base.desc_table_size());

        base.init_tail_head();
        base.write_reg(TXDCTL, (TXDCTL::ENABLE::SET + TXDCTL::WTHRESH.val(1)).value);
        while base.read_reg::<u32>(TXDCTL) & TXDCTL::ENABLE::SET.value == 0 {}
        debug!("TX ring initialized successfully");
        let mut meta_ls = Vec::with_capacity(desc_n);
        for _ in 0..desc_n {
            meta_ls.push(None);
        }
        Self { base, meta_ls }
    }
    pub fn transmit(&mut self, p: Pkt) -> Result<usize, ()> {
        // clear out 1 used tx desc in hardware
        if let Some((desc, idx)) = self.base.get_available() {
            debug!("clear out desc @ {}, done: {}", idx, unsafe {
                desc.write.is_done()
            });
            self.meta_ls[idx].take().expect("should have value");
        }
        if let Ok(tail) = self.base.add_desc(TxDesc::new(
            p.bus_addr(),
            p.buff.len(),
            TxAdvDescType::Data,
            &[
                TxAdvDescCmd::EOP,
                TxAdvDescCmd::RS,
                TxAdvDescCmd::IFCS,
                TxAdvDescCmd::DEXT,
            ],
        )) {
            self.meta_ls[tail] = Some(p);
            Ok(1)
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct TxDescRead {
    pub buffer_addr: u64,
    pub cmd_type_len: u32,
    pub olinfo_status: u32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct TxDescWriteBack {
    pub rsvd: u64,
    pub nxtseq_seed: u32,
    pub status: u32,
}

#[repr(C)]
pub union TxDesc {
    read: TxDescRead,
    write: TxDescWriteBack,
}

impl Descriptor for TxDesc {}

#[derive(Debug, Clone, Copy)]
pub enum TxAdvDescType {
    Data,
    #[allow(dead_code)]
    Context,
}

#[allow(dead_code)]
#[allow(clippy::upper_case_acronyms)]
pub enum TxAdvDescCmd {
    EOP,
    IFCS,
    IC,
    RS,
    DEXT,
    VLE,
    IDE,
}

register_bitfields![u32,
    // Advanced Transmit Descriptor CMD_TYPE_LEN field
    pub TX_DESC_CMD_TYPE_LEN [
        LEN OFFSET(0) NUMBITS(20)[],        // Packet Length [19:0]
        DTYPE OFFSET(20) NUMBITS(4)[
            Data = 0b11,                    // Data descriptor
            Context = 0b10,                 // Context descriptor
        ],
        CMD_EOP OFFSET(24) NUMBITS(1)[],    // End of Packet
        CMD_IFCS OFFSET(25) NUMBITS(1)[],   // Insert FCS
        CMD_IC OFFSET(26) NUMBITS(1)[],     // Insert Checksum
        CMD_RS OFFSET(27) NUMBITS(1)[],     // Report Status
        CMD_DEXT OFFSET(29) NUMBITS(1)[],   // Descriptor Extension
        CMD_VLE OFFSET(30) NUMBITS(1)[],    // VLAN Packet Enable
        CMD_IDE OFFSET(31) NUMBITS(1)[],    // Interrupt Delay Enable
    ],

    // Advanced Transmit Descriptor Status field (write-back format)
    pub TX_DESC_STATUS [
        DD OFFSET(0) NUMBITS(1)[],          // Descriptor Done
    ],
];

impl TxDesc {
    /// 创建新的发送描述符
    pub fn new(
        buffer_addr: u64,
        buffer_len: usize,
        kind: TxAdvDescType,
        cmd_ls: &[TxAdvDescCmd],
    ) -> Self {
        let mut cmd_type_len = TX_DESC_CMD_TYPE_LEN::LEN.val(buffer_len as _);
        match kind {
            TxAdvDescType::Data => {
                cmd_type_len += TX_DESC_CMD_TYPE_LEN::DTYPE::Data;
            }
            TxAdvDescType::Context => {
                cmd_type_len += TX_DESC_CMD_TYPE_LEN::DTYPE::Context;
            }
        }

        for c in cmd_ls {
            match c {
                TxAdvDescCmd::EOP => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_EOP::SET,
                TxAdvDescCmd::IFCS => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_IFCS::SET,
                TxAdvDescCmd::IC => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_IC::SET,
                TxAdvDescCmd::RS => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_RS::SET,
                TxAdvDescCmd::DEXT => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_DEXT::SET,
                TxAdvDescCmd::VLE => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_VLE::SET,
                TxAdvDescCmd::IDE => cmd_type_len += TX_DESC_CMD_TYPE_LEN::CMD_IDE::SET,
            }
        }

        Self {
            read: TxDescRead {
                buffer_addr,
                cmd_type_len: cmd_type_len.value,
                olinfo_status: 0,
            },
        }
    }
}

impl TxDescWriteBack {
    /// 检查描述符是否已完成 (DD bit)
    pub fn is_done(&self) -> bool {
        self.status & TX_DESC_STATUS::DD.mask != 0
    }
}
