
// virtio 相关虚拟地址配置信息(起始地址)

pub const MMIO: &[(usize, usize)] = &[
    (0x1000, 0x11000),          // VIRT_MROM
    (0x10008000, 0x1000),       // VIRT_NET
    (0xc000000, 0x4000000),     // VIRT_PLIC
];

// [VIRT_DEBUG] =       {        0x0,         0x100 },
// [VIRT_MROM] =        {     0x1000,       0x11000 },
// [VIRT_TEST] =        {   0x100000,        0x1000 },
// [VIRT_RTC] =         {   0x101000,        0x1000 },
// [VIRT_CLINT] =       {  0x2000000,       0x10000 },
// [VIRT_PLIC] =        {  0xc000000,     0x4000000 },
// [VIRT_UART0] =       { 0x10000000,         0x100 },
// [VIRT_VIRTIO] =      { 0x10001000,        0x1000 },
// [VIRT_UART1] =       { 0x10002000,         0x100 },
// [VIRT_UART2] =       { 0x10003000,         0x100 },
// [VIRT_UART3] =       { 0x10004000,         0x100 },
// [VIRT_UART4] =       { 0x10005000,         0x100 },
// [VIRT_FLASH] =       { 0x20000000,     0x4000000 },
// [VIRT_DRAM] =        { 0x80000000,           0x0 },
// [VIRT_PCIE_MMIO] =   { 0x40000000,    0x40000000 },
// [VIRT_PCIE_PIO] =    { 0x03000000,    0x00010000 },
// [VIRT_PCIE_ECAM] =   { 0x30000000,    0x10000000 },