use core::{fmt::Display, ptr::NonNull, time::Duration};

use mbarrier::mb;
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

use crate::{Speed, misc::wait_for};

register_structs! {
    pub MacRegister {
        (0x0 => ctrl: ReadWrite<u32, CTRL::Register>),
        (0x4 => _rsv1),
        (0x8 => status: ReadOnly<u32, STATUS::Register>),
        (0xC => _rsv2),
        (0x18 => ctrl_ext: ReadWrite<u32, CTRL_EXT::Register>),
        (0x1c => _rsv3),
        (0x20 => mdic: ReadWrite<u32, MDIC::Register>),
        (0x24 => _rsv4),
        (0x100 => pub rctl: ReadWrite<u32, RCTL::Register>),
        (0x104 => _rsv7),
        (0x400 => tctl: ReadWrite<u32,TCTL::Register>),
        (0x404 => _rsv12),
        (0x01524 => eims: ReadWrite<u32>),
        (0x01528 => eimc: ReadWrite<u32>),
        (0x0152c => eiac: ReadWrite<u32>),
        (0x01530 => eiam: ReadWrite<u32>),
        (0x01534 => _rsv5),
        (0x01580 => eicr: ReadWrite<u32>),
        (0x01584 => _rsv6),
        (0x5400 => ralh_0_15: [ReadWrite<u32>; 32]),
        (0x5480 => _rsv8),
        (0x54e0 => ralh_16_23: [ReadWrite<u32>;32]),
        (0x5560 => _rsv9),
        (0x5B50 => swsm: ReadWrite<u32, SWSM::Register>),
        (0x5B54 => fwsm: ReadWrite<u32>),
        (0x5B58 => _rsv10),
        (0x5B5C => sw_fw_sync: ReadWrite<u32>),
        (0x5B60 => _rsv11),

        // The end of the struct is marked as follows.
        (0xBFFF => @END),
    }
}

register_bitfields! [
    // First parameter is the register width. Can be u8, u16, u32, or u64.
    u32,

    CTRL [
        FD OFFSET(0) NUMBITS(1)[
            HalfDuplex = 0,
            FullDuplex = 1,
        ],
        SLU OFFSET(6) NUMBITS(1)[],
        SPEED OFFSET(8) NUMBITS(2)[
            Speed10 = 0,
            Speed100 = 1,
            Speed1000 = 0b10,
        ],
        FRCSPD OFFSET(11) NUMBITS(1)[],
        FRCDPLX OFFSET(12) NUMBITS(1)[],
        RST OFFSET(26) NUMBITS(1)[
            Normal = 0,
            Reset = 1,
        ],
        PHY_RST OFFSET(31) NUMBITS(1)[],
    ],
    STATUS [
        FD OFFSET(0) NUMBITS(1)[
            HalfDuplex = 0,
            FullDuplex = 1,
        ],
        LU OFFSET(1) NUMBITS(1)[],
        SPEED OFFSET(6) NUMBITS(2)[
            Speed10 = 0,
            Speed100 = 1,
            Speed1000 = 0b10,
        ],
        PHYRA OFFSET(10) NUMBITS(1)[],
    ],
    pub CTRL_EXT [
        LINK_MODE OFFSET(22) NUMBITS(2)[
            DircetCooper = 0,
            SGMII = 0b10,
            InternalSerdes = 0b11,
        ],
    ],
    MDIC [
        DATA OFFSET(0) NUMBITS(16)[],
        REGADDR OFFSET(16) NUMBITS(5)[],
        PHY_ADDR OFFSET(21) NUMBITS(5)[],
        OP OFFSET(26) NUMBITS(2)[
            Write = 0b1,
            Read = 0b10,
        ],
        READY OFFSET(28) NUMBITS(1)[],
        I OFFSET(29) NUMBITS(1)[],
        E OFFSET(30) NUMBITS(1)[
            NoError = 0,
            Error = 1,
        ],
        Destination OFFSET(31) NUMBITS(1)[
            Internal = 0,
            External = 1,
        ]
    ],

    SWSM [
        SMBI OFFSET(0) NUMBITS(1)[],
        SWESMBI OFFSET(1) NUMBITS(1)[],
        WMNG OFFSET(2) NUMBITS(1)[],
        EEUR OFFSET(3) NUMBITS(1)[],
    ],

    SW_FW_SYNC [
        SW_EEP_SM OFFSET(0) NUMBITS(1)[],
        SW_PHY_SM0 OFFSET(1) NUMBITS(1)[],
        SW_PHY_SM1 OFFSET(2) NUMBITS(1)[],
        SW_MAC_CSR_SM OFFSET(3) NUMBITS(1)[],
        SW_FLASH_SM OFFSET(4) NUMBITS(1)[],

        FW_EEP_SM OFFSET(16) NUMBITS(1)[],
        FW_PHY_SM0 OFFSET(17) NUMBITS(1)[],
        FW_PHY_SM1 OFFSET(18) NUMBITS(1)[],
        FW_MAC_CSR_SM OFFSET(19) NUMBITS(1)[],
        FW_FLASH_SM OFFSET(20) NUMBITS(1)[],
    ],

    pub RCTL [
        RXEN OFFSET(1) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        SBP OFFSET(2) NUMBITS(1)[
            DoNotStore = 0,
            Store = 1,
        ],
        UPE OFFSET(3) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        MPE OFFSET(4) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        LPE OFFSET(5) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        LBM OFFSET(6) NUMBITS(2)[
            Normal = 0b00,
            MacLoopback = 0b01,
            Reserved = 0b11,
        ],
        MO OFFSET(12) NUMBITS(2)[
            Bits47_36 = 0b00,
            Bits46_35 = 0b01,
            Bits45_34 = 0b10,
            Bits43_32 = 0b11,
        ],
        BAM OFFSET(15) NUMBITS(1)[
            Ignore = 0,
            Accept = 1,
        ],
        BSIZE OFFSET(16) NUMBITS(2)[
            Bytes2048 = 0b00,
            Bytes1024 = 0b01,
            Bytes512 = 0b10,
            Bytes256 = 0b11,
        ],
        VFE OFFSET(18) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        CFIEN OFFSET(19) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        CFI OFFSET(20) NUMBITS(1)[
            Accept = 0,
            Discard = 1,
        ],
        PSP OFFSET(21) NUMBITS(1)[],
        DPF OFFSET(22) NUMBITS(1)[
            Forward = 0,
            Discard = 1,
        ],
        PMCF OFFSET(23) NUMBITS(1)[
            Pass = 0,
            Filter = 1,
        ],
        SECRC OFFSET(26) NUMBITS(1)[
            DoNotStrip = 0,
            Strip = 1,
        ],
    ],
    // Transmit Control Register - TCTL (0x400)
    TCTL [
        EN OFFSET(1) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        PSP OFFSET(3) NUMBITS(1)[
            Disabled = 0,
            Enabled = 1,
        ],
        CT OFFSET(4) NUMBITS(8)[],
        COLD OFFSET(12) NUMBITS(10)[],
        SWXOFF OFFSET(22) NUMBITS(1)[],
        RTLC OFFSET(24) NUMBITS(1)[],
        NRTU OFFSET(25) NUMBITS(1)[],
        MULR OFFSET(28) NUMBITS(1)[],
    ],
];

#[derive(Clone, Copy)]
pub struct Mac {
    reg: NonNull<MacRegister>,
}

impl Mac {
    pub const fn new(ptr: NonNull<u8>) -> Self {
        Self { reg: ptr.cast() }
    }
    fn regs_mut(&mut self) -> &mut MacRegister {
        unsafe { self.reg.as_mut() }
    }
    fn regs(&self) -> &MacRegister {
        unsafe { self.reg.as_ref() }
    }
    pub fn base_addr<T>(&self) -> NonNull<T> {
        self.reg.cast()
    }
    pub fn disable_irq(&mut self) {
        self.regs_mut().eimc.set(u32::MAX);
    }

    pub fn reset(&mut self) -> Result<(), ()> {
        self.regs_mut()
            .ctrl
            .write(CTRL::RST::Reset + CTRL::PHY_RST::SET);
        wait_for(
            || {
                self.regs_mut()
                    .ctrl
                    .matches_all(CTRL::RST::Normal + CTRL::PHY_RST::CLEAR)
            },
            Duration::from_millis(100),
            Some(100),
        )
        .map_err(|_| ())
    }
    pub fn link_up(&mut self) {
        self.regs_mut().ctrl.modify(CTRL::SLU::SET);
    }
    pub fn mdic_read(&mut self, phy_addr: u32, offset: u32) -> Result<u16, ()> {
        self.regs_mut().mdic.write(
            MDIC::REGADDR.val(offset)
                + MDIC::PHY_ADDR.val(phy_addr)
                + MDIC::OP::Read
                + MDIC::READY::CLEAR
                + MDIC::I::CLEAR
                + MDIC::Destination::Internal,
        );
        mb();
        loop {
            let mdic = self.regs().mdic.extract();
            if mdic.is_set(MDIC::READY) {
                return Ok(mdic.read(MDIC::DATA) as _);
            }
            if mdic.is_set(MDIC::E) {
                return Err(());
            }
        }
    }
    pub fn mdic_write(&mut self, phy_addr: u32, offset: u32, data: u16) -> Result<(), ()> {
        self.regs_mut().mdic.write(
            MDIC::REGADDR.val(offset)
                + MDIC::PHY_ADDR.val(phy_addr)
                + MDIC::OP::Write
                + MDIC::READY::CLEAR
                + MDIC::I::CLEAR
                + MDIC::Destination::Internal
                + MDIC::DATA.val(data as _),
        );
        mb();
        loop {
            let mdic = self.regs().mdic.extract();
            if mdic.is_set(MDIC::READY) {
                return Ok(());
            }
            if mdic.is_set(MDIC::E) {
                return Err(());
            }
        }
    }
    pub fn status(&self) -> MacStatus {
        let status = self.regs().status.extract();
        let speed = match status.read_as_enum(STATUS::SPEED) {
            Some(STATUS::SPEED::Value::Speed1000) => Speed::Mb1000,
            Some(STATUS::SPEED::Value::Speed100) => Speed::Mb100,
            _ => Speed::Mb10,
        };
        MacStatus {
            speed,
            link_up: status.is_set(STATUS::LU),
            full_duplex: status.is_set(STATUS::FD),
            phy_reset_asserted: status.is_set(STATUS::PHYRA),
        }
    }
    pub fn enable_rx_tx(&mut self) {
        self.regs().rctl.modify(RCTL::RXEN::SET);
        self.regs().tctl.modify(TCTL::EN::Enabled);
    }

    pub fn mac_addr(&self) -> MacAddr {
        let mac_l = self.regs().ralh_0_15[0].get();
        let mac_h = self.regs().ralh_0_15[1].get();
        ((mac_h as u64) << 32 | mac_l as u64).into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MacStatus {
    pub speed: Speed,
    pub link_up: bool,
    pub full_duplex: bool,
    pub phy_reset_asserted: bool,
}
#[derive(Debug)]
pub struct MacAddr([u8; 6]);

impl Display for MacAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl MacAddr {
    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddr(bytes)
    }

    pub fn bytes(&self) -> [u8; 6] {
        self.0
    }
}

impl From<u64> for MacAddr {
    fn from(value: u64) -> Self {
        MacAddr([
            (value & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
        ])
    }
}
