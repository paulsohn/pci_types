use bit_field::BitField;
use core::fmt;

/// The address of a PCIe function.
///
/// PCIe supports 65536 segment groups, each with 256 buses, each with 32 slots, each with 8 possible functions. We pack this into a `u32`:
///
/// ```ignore
/// 32                              16               8         3      0
///  +-------------------------------+---------------+---------+------+
///  |             group             |      bus      |   slot  | func |
///  +-------------------------------+---------------+---------+------+
/// ```
/// 
/// In legacy PCI systems, only segment group 0 is available.
/// 
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct PciAddress(u32);

impl PciAddress {
    pub fn new(group: u16, bus: u8, device: u8, function: u8) -> PciAddress {
        PciAddress(
            *0.set_bits(16..=31, group as u32)
                .set_bits(8..=15, bus as u32)
                .set_bits(3..=7, device as u32)
                .set_bits(0..=2, function as u32)
        )
    }

    /// The segment group number.
    pub fn group(&self) -> u16 {
        self.0.get_bits(16..=31) as u16
    }

    #[deprecated = "use `.group()` insetad."]
    /// The segment group number.
    pub fn segment(&self) -> u16 {
        self.group()
    }

    /// The bus segment number.
    pub fn bus(&self) -> u8 {
        self.0.get_bits(8..=15) as u8
    }

    /// The device slot number.
    pub fn slot(&self) -> u8 {
        self.0.get_bits(3..=7) as u8
    }

    #[deprecated = "use `.slot()` instead."]
    /// The device slot number.
    pub fn device(&self) -> u8 {
        self.slot()
    }

    /// The function number.
    pub fn function(&self) -> u8 {
        self.0.get_bits(0..=2) as u8
    }
}

impl fmt::Display for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}-{:02x}:{:02x}.{}", self.group(), self.bus(), self.slot(), self.function())
    }
}

impl fmt::Debug for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

// TODO: documentation
pub trait DwordAccessMethod: Clone {
    fn function_exists(&self, address: PciAddress) -> bool;
    fn has_multiple_functions(&self, address: PciAddress) -> bool;
    unsafe fn read_dword(&self, address: PciAddress, offset: u16) -> u32;
    unsafe fn write_dword(&self, address: PciAddress, offset: u16, value: u32);
}