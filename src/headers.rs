//! PCI configuration space headers.

use crate::dwords::*;
use crate::address::DwordAccessMethod;
use crate::accessor::AccessorTrait;
use crate::capability::{CapabilityHeader, CapabilityIterator};

use crate::map_field;

use bit_field::BitField;

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderType {
    Endpoint,
    PciPciBridge,
    CardBusBridge,
    Unknown(u8),
}

/// Every PCI configuration region starts with a header made up of two parts:
///    - a predefined region that identify the function (bytes `0x00..0x10`)
///    - a device-dependent region that depends on the Header Type field
///
/// The predefined region is of the form:
/// ```ignore
///     32                            16                              0
///      +-----------------------------+------------------------------+
///      |       Device ID             |       Vendor ID              | 0x00
///      |                             |                              |
///      +-----------------------------+------------------------------+
///      |         Status              |       Command                | 0x04
///      |                             |                              |
///      +-----------------------------+---------------+--------------+
///      |               Class Code                    |   Revision   | 0x08
///      |                                             |      ID      |
///      +--------------+--------------+---------------+--------------+
///      |     BIST     |    Header    |    Latency    |  Cacheline   | 0x0c
///      |              |     type     |     timer     |    size      |
///      +--------------+--------------+---------------+--------------+
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PciHeader {
    /// Device ID registers. Offset 0x00.
    pub ids: IDs,
    /// Command and Status registers. Offset 0x04.
    pub cmdsts: CmdSts,
    /// Revision ID and Class Code registers. Offset 0x08.
    pub revcc: RevCC,
    /// Cacheline Size, Latency Timer, Header Type, Multi-fn support bit, BIST.
    /// Offset 0x0C.
    pub config: HeaderTypeDword,
}

/// Endpoints have a Type-0 header, so the remainder of the header is of the form:
/// ```ignore
///     32                           16                              0
///     +-----------------------------------------------------------+ 0x00
///     |                                                           |
///     |                Predefined region of header                |
///     |                                                           |
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 0                  | 0x10
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 1                  | 0x14
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 2                  | 0x18
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 3                  | 0x1c
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 4                  | 0x20
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 5                  | 0x24
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  CardBus CIS Pointer                      | 0x28
///     |                                                           |
///     +----------------------------+------------------------------+
///     |       Subsystem ID         |    Subsystem vendor ID       | 0x2c
///     |                            |                              |
///     +----------------------------+------------------------------+
///     |               Expansion ROM Base Address                  | 0x30
///     |                                                           |
///     +--------------------------------------------+--------------+
///     |                 Reserved                   | Capabilities | 0x34
///     |                                            |   Pointer    |
///     +--------------------------------------------+--------------+
///     |                         Reserved                          | 0x38
///     |                                                           |
///     +--------------+--------------+--------------+--------------+
///     |   Max_Lat    |   Min_Gnt    |  Interrupt   |  Interrupt   | 0x3c
///     |              |              |   PIN        |   Line       |
///     +--------------+--------------+--------------+--------------+
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EndpointHeader {
    /// Predefined PCI header. Offset 0x00.
    pub header: PciHeader,
    bar_raw: [u32; 6],
    /// CardBus CIS pointer register. Offset 0x28.
    pub cardbus_cis_pointer: u32,
    /// Subsystem ID registers. Offset 0x2C.
    pub subsystem_ids: IDs,
    /// Expansion ROM Base Address register. Offset 0x30.
    pub expansion_rom_base_address: u32,
    capabilities_pointer: u8,
    _reserved_35_3b: [u8; 7],
    /// Interrupt Line/PIN registers. Offset 0x3C.
    pub interrupt: Intr,
    /// Min Grant / Max Latency registers. Offset 0x3E.
    pub limits: EPLimits,
}
impl EndpointHeader {
    // accessor-based methods.
    // todo: common implementations of from_header, capabilities, reading and writing bars for EP header and Pci-to-Pci header.

    pub fn from_header<'a, M>(header_acc: &impl AccessorTrait<'a, M, PciHeader>) -> Option<impl AccessorTrait<'a, M, Self>>
    where
        M: DwordAccessMethod,
    {
        if map_field!(header_acc.config).read().header_type() == Ok(HeaderType::Endpoint) {
            Some(header_acc.cast())
        } else { None }
    }

    pub fn capabilities<'a, M>(acc: &impl AccessorTrait<'a, M, Self>)
    -> CapabilityIterator<'a, M, impl AccessorTrait<'a, M, CapabilityHeader>>
    where
        M: DwordAccessMethod,
    {
        let first_offset = if map_field!(acc.header.cmdsts).read()
            .status().has_capability_list()
        {
            map_field!(acc.capabilities_pointer).read()
        } else {
            0u8 // if offset zero, then the iterator is empty.
        };

        CapabilityIterator::new(
            acc.cast().with_offset(first_offset as u16)
        )
    }

    pub const MAX_BARS: usize = 6;

    /// Get the contents of a BAR slot, or two slots if the BAR occupies 64 bits.
    /// Empty or invalid bars will return `None`.
    /// 
    /// If the result of reading BAR slot #`x` is 64-bit memory, then this method should not be called for slot #`x+1`.
    /// 
    /// This exists as an associated function, as reading BAR involves reading back through the accessor.
    pub fn bar<'a, M>(acc: &impl AccessorTrait<'a, M, Self>, slot: u8) -> Option<Bar>
    where
        M: DwordAccessMethod,
    {
        // todo: 
        // "Before attempting to read the information about the BAR, make sure to disable both I/O and memory decode in the command byte. You can restore the original value after completing the BAR info read."
    
        if slot as usize >= Self::MAX_BARS {
            return None;
        }

        let bar_acc = map_field!(acc.bar_raw[slot as usize]);
        let bar = bar_acc.read();
    
        if bar.get_bit(0) == false {
            let prefetchable = bar.get_bit(3);
            let address = bar.get_bits(4..32) << 4;
    
            match bar.get_bits(1..3) {
                0b00 => {
                    let size = {
                        bar_acc.write(0xfffffff0);
                        let mut readback = bar_acc.read();
                        bar_acc.write(address);
    
                        /*
                         * If the entire readback value is zero, the BAR is not implemented, so we return `None`.
                         */
                        if readback == 0x0 {
                            return None;
                        }
    
                        readback.set_bits(0..4, 0);
                        1 << readback.trailing_zeros()
                    };
                    Some(Bar::Memory32 { address, size, prefetchable })
                }
    
                0b10 => {
                    /*
                     * If the BAR is 64 bit-wide and this slot is the last, there is no second slot to read.
                     */
                    if slot as usize >= Self::MAX_BARS - 1 {
                        return None;
                    }
    
                    let next_bar_acc = map_field!(acc.bar_raw[(slot + 1) as usize]);
    
                    let address_upper: u32 = next_bar_acc.read();
    
                    let size = {
                        bar_acc.write(0xfffffff0);
                        next_bar_acc.write(0xffffffff);
    
                        let mut readback_low = bar_acc.read();
                        let readback_high = next_bar_acc.read();
    
                        bar_acc.write(address);
                        next_bar_acc.write(address_upper);
    
                        /*
                         * If the readback from the first slot is not 0, the size of the BAR is less than 4GiB.
                         */
                        readback_low.set_bits(0..4, 0);
                        if readback_low != 0 {
                            (1 << readback_low.trailing_zeros()) as u64
                        } else {
                            1u64 << ((readback_high.trailing_zeros() + 32) as u64)
                        }
                    };
    
                    let address = {
                        let mut address = address as u64;
                        address.set_bits(32..64, address_upper as u64);
                        address
                    };
    
                    Some(Bar::Memory64 { address, size, prefetchable })
                }
                // TODO: should we bother to return an error here?
                _ => panic!("BAR Memory type is reserved!"),
            }
        } else {
            Some(Bar::Io { port: bar.get_bits(2..32) << 2 })
        }
    }

    /// Similar to `Self::bar`, but this don't read back and the size is zero.
    pub fn bar_base_only<'a, M>(acc: &impl AccessorTrait<'a, M, Self>, slot: u8) -> Option<Bar>
    where
        M: DwordAccessMethod,
    {
        // todo: 
        // "Before attempting to read the information about the BAR, make sure to disable both I/O and memory decode in the command byte. You can restore the original value after completing the BAR info read."
    
        if slot as usize >= Self::MAX_BARS {
            return None;
        }

        let bar_acc = map_field!(acc.bar_raw[slot as usize]);
        let bar = bar_acc.read();
    
        if bar.get_bit(0) == false {
            let prefetchable = bar.get_bit(3);
            let address = bar.get_bits(4..32) << 4;
    
            match bar.get_bits(1..3) {
                0b00 => {
                    Some(Bar::Memory32 { address, size: 0, prefetchable })
                }
    
                0b10 => {
                    /*
                     * If the BAR is 64 bit-wide and this slot is the last, there is no second slot to read.
                     */
                    if slot as usize >= Self::MAX_BARS - 1 {
                        return None;
                    }
    
                    let next_bar_acc = map_field!(acc.bar_raw[(slot + 1) as usize]);
    
                    let address_upper: u32 = next_bar_acc.read();
                    let address = {
                        let mut address = address as u64;
                        address.set_bits(32..64, address_upper as u64);
                        address
                    };
    
                    Some(Bar::Memory64 { address, size: 0, prefetchable })
                }
                // TODO: should we bother to return an error here?
                _ => panic!("BAR Memory type is reserved!"),
            }
        } else {
            Some(Bar::Io { port: bar.get_bits(2..32) << 2 })
        }
    }

    pub unsafe fn write_bar<'a, M>(
        acc: &impl AccessorTrait<'a, M, Self>,
        slot: u8,
        value: usize,
    ) -> Result<(), BarWriteError>
    where
        M: DwordAccessMethod,
    {
        match Self::bar(acc, slot) {
            Some(Bar::Memory64 { .. }) => {
                map_field!(acc.bar_raw[slot as usize]).write(
                    value.get_bits(0..=31) as u32
                );
                map_field!(acc.bar_raw[(slot + 1) as usize]).write(
                    value.get_bits(32..=63) as u32
                );
                Ok(())
            },
            Some(_) => { // Memory32 or Io
                if value > u32::MAX as usize {
                    return Err(BarWriteError::InvalidValue);
                }
                map_field!(acc.bar_raw[slot as usize]).write(
                    value as u32
                );
                Ok(())
            }
            None => return Err(BarWriteError::NoSuchBar),
        }
    }
}

/// PCI-PCI Bridges have a Type-1 header, so the remainder of the header is of the form:
/// ```ignore
///     32                           16                              0
///     +-----------------------------------------------------------+ 0x00
///     |                                                           |
///     |                Predefined region of header                |
///     |                                                           |
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 0                  | 0x10
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 1                  | 0x14
///     |                                                           |
///     +--------------+--------------+--------------+--------------+
///     | Secondary    | Subordinate  |  Secondary   | Primary Bus  | 0x18
///     |Latency Timer | Bus Number   |  Bus Number  |   Number     |
///     +--------------+--------------+--------------+--------------+
///     |      Secondary Status       |  I/O Limit   |   I/O Base   | 0x1C
///     |                             |              |              |
///     +-----------------------------+--------------+--------------+
///     |        Memory Limit         |         Memory Base         | 0x20
///     |                             |                             |
///     +-----------------------------+-----------------------------+
///     |  Prefetchable Memory Limit  |  Prefetchable Memory Base   | 0x24
///     |                             |                             |
///     +-----------------------------+-----------------------------+
///     |             Prefetchable Base Upper 32 Bits               | 0x28
///     |                                                           |
///     +-----------------------------------------------------------+
///     |             Prefetchable Limit Upper 32 Bits              | 0x2C
///     |                                                           |
///     +-----------------------------+-----------------------------+
///     |   I/O Limit Upper 16 Bits   |   I/O Base Upper 16 Bits    | 0x30
///     |                             |                             |
///     +-----------------------------+--------------+--------------+
///     |              Reserved                      |  Capability  | 0x34
///     |                                            |   Pointer    |
///     +--------------------------------------------+--------------+
///     |                  Expansion ROM Base Address               | 0x38
///     |                                                           |
///     +-----------------------------+--------------+--------------+
///     |    Bridge Control           |  Interrupt   | Interrupt    | 0x3C
///     |                             |     PIN      |   Line       |
///     +-----------------------------+--------------+--------------+
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PciPciBridgeHeader {
    /// Predefined PCI header. Offset 0x00.
    pub header: PciHeader,
    bar_raw: [u32; 2],
    /// Primary / Secondary Bus Number Registers.
    pub bus_nos: BusNos,
    
    pub iobase: u8,
    pub iolimit: u8,
    pub secondary_status: Status, // todo: dword

    pub memory_base: u16,
    pub memory_limit: u16, // todo: dword

    pub prefetchable_memory_base: u16,
    pub prefetchable_memory_limit: u16, // todo: dword

    pub prefetchable_base_upper_32: u32,
    pub prefetchable_limit_upper_32: u32,

    pub iobase_upper_16: u16,
    pub iolimit_upper_16: u16, // todo: dword

    capabilities_pointer: u8,
    _reserved_35_3b: [u8; 3],
    pub expansion_rom_base_address: u32,

    /// Interrupt Line/PIN registers. Offset 0x3C.
    pub interrupt: Intr,
    /// Min Grant / Max Latency registers. Offset 0x3E.
    pub bridge_control: BridgeControl,
}
impl PciPciBridgeHeader {
    pub fn from_header<'a, M>(header_acc: &impl AccessorTrait<'a, M, PciHeader>) -> Option<impl AccessorTrait<'a, M, Self>>
    where
        M: DwordAccessMethod,
    {
        if map_field!(header_acc.config).read().header_type() == Ok(HeaderType::PciPciBridge) {
            Some(header_acc.cast())
        } else { None }
    }

    pub fn capabilities<'a, M>(acc: &impl AccessorTrait<'a, M, Self>)
    -> CapabilityIterator<'a, M, impl AccessorTrait<'a, M, CapabilityHeader>>
    where
        M: DwordAccessMethod,
    {
        let first_offset = if map_field!(acc.header.cmdsts).read()
            .status().has_capability_list()
        {
            map_field!(acc.capabilities_pointer).read()
        } else {
            0u8 // if offset zero, then the iterator is empty.
        };

        CapabilityIterator::new(
            acc.cast().with_offset(first_offset as u16)
        )
    }

    pub const MAX_BARS: usize = 2;

    /// Get the contents of a BAR slot, or two slots if the BAR occupies 64 bits.
    /// Empty or invalid bars will return `None`.
    /// 
    /// If the result of reading BAR slot #`x` is 64-bit memory, then this method should not be called for slot #`x+1`.
    /// 
    /// This exists as an associated function, as reading BAR involves reading back through the accessor.
    pub fn bar<'a, M>(acc: &impl AccessorTrait<'a, M, Self>, slot: u8) -> Option<Bar>
    where
        M: DwordAccessMethod,
    {
        // todo: 
        // "Before attempting to read the information about the BAR, make sure to disable both I/O and memory decode in the command byte. You can restore the original value after completing the BAR info read."
    
        if slot as usize >= Self::MAX_BARS {
            return None;
        }

        let bar_acc = map_field!(acc.bar_raw[slot as usize]);
        let bar = bar_acc.read();
    
        if bar.get_bit(0) == false {
            let prefetchable = bar.get_bit(3);
            let address = bar.get_bits(4..32) << 4;
    
            match bar.get_bits(1..3) {
                0b00 => {
                    let size = {
                        bar_acc.write(0xfffffff0);
                        let mut readback = bar_acc.read();
                        bar_acc.write(address);
    
                        /*
                         * If the entire readback value is zero, the BAR is not implemented, so we return `None`.
                         */
                        if readback == 0x0 {
                            return None;
                        }
    
                        readback.set_bits(0..4, 0);
                        1 << readback.trailing_zeros()
                    };
                    Some(Bar::Memory32 { address, size, prefetchable })
                }
    
                0b10 => {
                    /*
                     * If the BAR is 64 bit-wide and this slot is the last, there is no second slot to read.
                     */
                    if slot as usize >= Self::MAX_BARS - 1 {
                        return None;
                    }
    
                    let next_bar_acc = map_field!(acc.bar_raw[(slot + 1) as usize]);
    
                    let address_upper: u32 = next_bar_acc.read();
    
                    let size = {
                        bar_acc.write(0xfffffff0);
                        next_bar_acc.write(0xffffffff);
    
                        let mut readback_low = bar_acc.read();
                        let readback_high = next_bar_acc.read();
    
                        bar_acc.write(address);
                        next_bar_acc.write(address_upper);
    
                        /*
                         * If the readback from the first slot is not 0, the size of the BAR is less than 4GiB.
                         */
                        readback_low.set_bits(0..4, 0);
                        if readback_low != 0 {
                            (1 << readback_low.trailing_zeros()) as u64
                        } else {
                            1u64 << ((readback_high.trailing_zeros() + 32) as u64)
                        }
                    };
    
                    let address = {
                        let mut address = address as u64;
                        address.set_bits(32..64, address_upper as u64);
                        address
                    };
    
                    Some(Bar::Memory64 { address, size, prefetchable })
                }
                // TODO: should we bother to return an error here?
                _ => panic!("BAR Memory type is reserved!"),
            }
        } else {
            Some(Bar::Io { port: bar.get_bits(2..32) << 2 })
        }
    }

    /// Similar to `Self::bar`, but this don't read back and the size is zero.
    pub fn bar_base_only<'a, M>(acc: &impl AccessorTrait<'a, M, Self>, slot: u8) -> Option<Bar>
    where
        M: DwordAccessMethod,
    {
        // todo: 
        // "Before attempting to read the information about the BAR, make sure to disable both I/O and memory decode in the command byte. You can restore the original value after completing the BAR info read."
    
        if slot as usize >= Self::MAX_BARS {
            return None;
        }

        let bar_acc = map_field!(acc.bar_raw[slot as usize]);
        let bar = bar_acc.read();
    
        if bar.get_bit(0) == false {
            let prefetchable = bar.get_bit(3);
            let address = bar.get_bits(4..32) << 4;
    
            match bar.get_bits(1..3) {
                0b00 => {
                    Some(Bar::Memory32 { address, size: 0, prefetchable })
                }
    
                0b10 => {
                    /*
                     * If the BAR is 64 bit-wide and this slot is the last, there is no second slot to read.
                     */
                    if slot as usize >= Self::MAX_BARS - 1 {
                        return None;
                    }
    
                    let next_bar_acc = map_field!(acc.bar_raw[(slot + 1) as usize]);
    
                    let address_upper: u32 = next_bar_acc.read();
                    let address = {
                        let mut address = address as u64;
                        address.set_bits(32..64, address_upper as u64);
                        address
                    };
    
                    Some(Bar::Memory64 { address, size: 0, prefetchable })
                }
                // TODO: should we bother to return an error here?
                _ => panic!("BAR Memory type is reserved!"),
            }
        } else {
            Some(Bar::Io { port: bar.get_bits(2..32) << 2 })
        }
    }

    pub unsafe fn write_bar<'a, M>(
        acc: &impl AccessorTrait<'a, M, Self>,
        slot: u8,
        value: usize,
    ) -> Result<(), BarWriteError>
    where
        M: DwordAccessMethod,
    {
        match Self::bar(acc, slot) {
            Some(Bar::Memory64 { .. }) => {
                map_field!(acc.bar_raw[slot as usize]).write(
                    value.get_bits(0..=31) as u32
                );
                map_field!(acc.bar_raw[(slot + 1) as usize]).write(
                    value.get_bits(32..=63) as u32
                );
                Ok(())
            },
            Some(_) => { // Memory32 or Io
                if value > u32::MAX as usize {
                    return Err(BarWriteError::InvalidValue);
                }
                map_field!(acc.bar_raw[slot as usize]).write(
                    value as u32
                );
                Ok(())
            }
            None => return Err(BarWriteError::NoSuchBar),
        }
    }
}

// todo: pci-cardbus bridge header

#[derive(Clone, Copy, Debug)]
pub enum Bar {
    Memory32 { address: u32, size: u32, prefetchable: bool },
    Memory64 { address: u64, size: u64, prefetchable: bool },
    Io { port: u32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BarWriteError {
    NoSuchBar,
    InvalidValue,
}