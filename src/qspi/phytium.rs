use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadWrite, WriteOnly},
};

register_bitfields! [
    u32,

    // FLASH 容量设置寄存器
    pub FlashCapacity [
        SIZE  OFFSET(0)  NUMBITS(3) [
            Bytes4M   = 0b000,
            Bytes8M   = 0b001,
            Bytes16M   = 0b010,
            Bytes32M   = 0b011,
            Bytes64M   = 0b110,
            Bytes4M2   = 0b111,
        ],
        NUM OFFSET(3) NUMBITS(2) [
            NUM1 = 0,
            NUM2 = 1,
            NUM3 = 2,
            NUM4 = 3,
        ],
    ],

    // 读配置寄存器
    RdCfg [
        READ_MODE    OFFSET(0)  NUMBITS(3) [
            Normal   = 0,
            FastRead = 1,
            Dual     = 2,
            Quad     = 3
        ],
        DUMMY_CYCLE  OFFSET(8)  NUMBITS(8) []
    ],

    // 写配置寄存器
    WrCfg [
        PAGE_PROGRAM OFFSET(0)  NUMBITS(1) [],
        SECTOR_ERASE OFFSET(1)  NUMBITS(1) [],
        CHIP_ERASE   OFFSET(2)  NUMBITS(1) []
    ],

    // 命令端口寄存器
    CmdPort [
        COMMAND   OFFSET(0)  NUMBITS(8) [],
        EXECUTE   OFFSET(31) NUMBITS(1) []
    ],

    // 地址端口寄存器
    AddrPort [
        ADDRESS   OFFSET(0)  NUMBITS(24) [],
        READ_ONLY OFFSET(31) NUMBITS(1) []
    ],

    // 数据端口寄存器
    DataPort [
        DATA      OFFSET(0)  NUMBITS(16) []
    ],

    // 片选设置寄存器
    CsSet [
        CHIP_SELECT OFFSET(0) NUMBITS(2) [
            CS0 = 0,
            CS1 = 1,
            CS2 = 2
        ],
        ACTIVE_HIGH OFFSET(7) NUMBITS(1) []
    ],

    // WIP 读取设置寄存器
    WipRd [
        POLLING     OFFSET(0)  NUMBITS(1) [],
        STATUS_REG  OFFSET(1)  NUMBITS(1) [],
        BUSY_BIT    OFFSET(8)  NUMBITS(8) []
    ],

    // 写保护设置寄存器
    WpReg [
        WRITE_PROTECT OFFSET(0)  NUMBITS(1) [],
        PROTECT_RANGE OFFSET(4)  NUMBITS(24) []
    ],

    // XIP 模式设置寄存器
    ModeReg [
        XIP_ENABLE   OFFSET(0)  NUMBITS(1) [],
        CACHE_SIZE   OFFSET(4)  NUMBITS(4) [
            Size0K = 0,
            Size1K = 1,
            Size2K = 2,
            Size4K = 4
        ],
        CACHE_THRESHOLD OFFSET(8) NUMBITS(8) []
    ],

    // 分频系数设置寄存器
    CycleReg [
        CLOCK_DIVIDER OFFSET(0) NUMBITS(8) [
            Div1   = 0,
            Div2   = 1,
            Div4   = 2,
            Div8   = 3
        ],
        TIMEOUT       OFFSET(16) NUMBITS(16) []
    ]
];

register_structs! {
    pub FlashControllerRegisters {
        (0x000 => flash_capacity: ReadWrite<u32, FlashCapacity::Register>),
        (0x004 => rd_cfg: ReadWrite<u32, RdCfg::Register>),
        (0x008 => wr_cfg: ReadWrite<u32, WrCfg::Register>),
        (0x00C => flush_reg: WriteOnly<u32>),
        (0x010 => cmd_port: ReadWrite<u32, CmdPort::Register>),
        (0x014 => addr_port: ReadWrite<u32, AddrPort::Register>),
        (0x018 => hd_port: ReadWrite<u32, DataPort::Register>),
        (0x01C => ld_port: ReadWrite<u32, DataPort::Register>),
        (0x020 => cs_set: ReadWrite<u32, CsSet::Register>),
        (0x024 => wip_rd: ReadWrite<u32, WipRd::Register>),
        (0x028 => wp_reg: ReadWrite<u32, WpReg::Register>),
        (0x02C => mode_reg: ReadWrite<u32, ModeReg::Register>),
        (0x030 => cycle_reg: ReadWrite<u32, CycleReg::Register>),
        (0x034 => @END),
    }
}
