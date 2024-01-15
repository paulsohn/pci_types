use crate::{
    capability::CapabilityHeader,
    DwordAccessMethod, AccessorTrait,
    map_field,
};
use bit_field::BitField;
use core::convert::TryFrom;

/// Specifies how many MSI interrupts one device can have.
/// Device will modify lower bits of interrupt vector to send multiple messages, so interrupt block
/// must be aligned accordingly.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u8)]
pub enum MultipleMessageSupport {
    /// Device can send 1 interrupt. No interrupt vector modification is happening here
    Int1 = 0b000,
    /// Device can send 2 interrupts
    Int2 = 0b001,
    /// Device can send 4 interrupts
    Int4 = 0b010,
    /// Device can send 8 interrupts
    Int8 = 0b011,
    /// Device can send 16 interrupts
    Int16 = 0b100,
    /// Device can send 32 interrupts
    Int32 = 0b101,
}

impl TryFrom<u8> for MultipleMessageSupport {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b000 => Ok(MultipleMessageSupport::Int1),
            0b001 => Ok(MultipleMessageSupport::Int2),
            0b010 => Ok(MultipleMessageSupport::Int4),
            0b011 => Ok(MultipleMessageSupport::Int8),
            0b100 => Ok(MultipleMessageSupport::Int16),
            0b101 => Ok(MultipleMessageSupport::Int32),
            _ => Err(()),
        }
    }
}

/// When device should trigger the interrupt
#[derive(Debug)]
pub enum TriggerMode {
    Edge = 0b00,
    LevelAssert = 0b11,
    LevelDeassert = 0b10,
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct MessageControl(u16);
impl MessageControl {
    pub fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub fn msi_enable(&self) -> bool {
        self.0.get_bit(0)
    }
    pub fn set_msi_enable(&mut self) {
        self.0.set_bit(0, true);
    }
    pub fn clear_msi_enable(&mut self) {
        self.0.set_bit(0, false);
    }

    pub fn multiple_message_capable(&self) -> MultipleMessageSupport {
        MultipleMessageSupport::try_from(self.0.get_bits(1..=3) as u8)
            .unwrap()
    }
    pub fn multiple_message_enable(&self) -> MultipleMessageSupport {
        MultipleMessageSupport::try_from(self.0.get_bits(4..=6) as u8)
            .unwrap_or(MultipleMessageSupport::Int1)
    }
    /// Set how many interrupts the device will use.
    /// If request count is greater than capability, then the capability will be used.
    pub fn set_multiple_message_enable(&mut self, value: MultipleMessageSupport) {
        self.0.set_bits(
            4..=6,
            value.min(self.multiple_message_capable()) as u16
        );
    }

    pub fn is_64bit(&self) -> bool {
        self.0.get_bit(7)
    }

    pub fn has_per_vector_masking(&self) -> bool {
        self.0.get_bit(8)
    }
}

/// MSI message data.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct MessageData(u32); // upper 16 bits should be rsvdz.
impl MessageData {
    pub fn new(vector: u8, trigger_mode: TriggerMode) -> Self {
        Self(
            *(vector as u32).set_bits(14..=15, trigger_mode as u32),
        )
    }
}

/// The generic MSI Capability Header.
/// 
/// The generic parameter `N` should be either 1 or 2.
/// Users are not supposed to feed `N` directly.
/// Instead, use [`MsiCapability32Bit`] and [`MsiCapability64Bit`].
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MsiCapability<const N: usize> { // todo: instead of using generics, construct separate type for 32-bit and 64-bit.
    /// Capability Header. Offset 0x00.
    pub header: CapabilityHeader<MessageControl>,
    /// Address Part, with one dword(32-bit) and two dwords(64-bit). Offset 0x04.
    pub addr: [u32; N],
    /// Message Data. Offset 0x08 (32-bit) / 0x0C (64-bit).
    pub data: MessageData,
    /// Mask Bits, only valid in 64-bit mode and when per-vector masking is enabled.
    /// Offset 0x10.
    pub mask_bits: u32,
    /// Pending Bits, only valid in 64-bit mode.
    /// Offset 0x14.
    pub pending_bits: u32,
}

pub type MsiCapability32Bit = MsiCapability<1>;
pub type MsiCapability64Bit = MsiCapability<2>;
pub type MsiCapabilityInfo = MsiCapability64Bit;

impl<const N: usize> MsiCapability<N> {
    /// Converts a cap header accessor into MSI cap accessor.
    /// 
    /// Note that there are actually TWO converter methods,
    /// namely `MsiCapability32Bit::from_header` and `MsiCapability64Bit::from_header`.
    /// 
    /// Since we can check the address size only once,
    /// using `read_info` and `write_info` are preferred,
    pub fn from_header<'a, M>(cap_header_acc: &impl AccessorTrait<'a, M, CapabilityHeader>) -> Option<impl AccessorTrait<'a, M, Self>>
    where
        M: DwordAccessMethod,
    {
        let header: CapabilityHeader = cap_header_acc.read();

        if header.id == 0x05 { // todo: define MSI Cap header ID = 0x05 somewhere
            let ctrl = MessageControl::new(header.extension);
            if ctrl.is_64bit() == (N != 1) {
                Some(cap_header_acc.cast())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl MsiCapabilityInfo {
    /// Determine if this header is MSI Capability.
    /// 0 if this is not MSI, 1 if 32-bit, 2 if 64-bit.
    pub fn msi_cap_type<'a, M>(cap_header_acc: &impl AccessorTrait<'a, M, CapabilityHeader>) -> usize
    where
        M: DwordAccessMethod,
    {
        let CapabilityHeader { id, next: _, extension } = cap_header_acc.read();
        if id != 0x05 { // todo: define MSI Cap header ID = 0x05 somewhere
            return 0;
        }

        let ctrl = MessageControl::new(extension);
        if !ctrl.is_64bit() { 1 } else { 2 }
    }

    /// Read the MSI Capability info from given cap header.
    /// 
    /// Panics if the header is not MSI header.
    pub fn read_info<'a, M>(cap_header_acc: &impl AccessorTrait<'a, M, CapabilityHeader>) -> Self
    where
        M: DwordAccessMethod,
    {
        let CapabilityHeader { id, next, extension } = cap_header_acc.read();
        assert_eq!(id, 0x05); // todo: define MSI Cap header ID = 0x05 somewhere

        let ctrl = MessageControl::new(extension);
        let header = CapabilityHeader::<MessageControl> { id, next, extension: ctrl };
        
        if !ctrl.is_64bit() { // 32-bit
            let casted = cap_header_acc.cast::<MsiCapability32Bit>();

            let addr0 = map_field!(casted.addr[0]).read();
            let data = map_field!(casted.data).read(); // padding should be zero, so no need to read.

            MsiCapability64Bit {
                header,
                addr: [addr0, 0],
                data,
                mask_bits: 0,
                pending_bits: 0,
            }
        } else {
            let casted = cap_header_acc.cast::<MsiCapability64Bit>();

            let addr0 = map_field!(casted.addr[0]).read();
            let addr1 = map_field!(casted.addr[1]).read();
            let data = map_field!(casted.data).read(); // padding should be zero, so no need to read.
            let mask_bits = map_field!(casted.mask_bits).read();
            let pending_bits = map_field!(casted.pending_bits).read();

            MsiCapability64Bit {
                header,
                addr: [addr0, addr1],
                data,
                mask_bits,
                pending_bits,
            }

            // // in this branch, this also works (read header twice though)
            // cap_header_acc.cast::<MsiCapability64Bit>().read()
        }
    }

    /// Write the MSI Capability info into given cap header.
    /// 
    /// Panics if the header is not MSI header.
    pub fn write_info<'a, M>(cap_header_acc: &impl AccessorTrait<'a, M, CapabilityHeader>, info: Self)
    where
        M: DwordAccessMethod,
    {
        let CapabilityHeader { id, next, extension } = cap_header_acc.read();
        assert_eq!(id, 0x05); // todo: define MSI Cap header ID = 0x05 somewhere
        assert_eq!(info.header.id, id);
        assert_eq!(info.header.next, next);

        let ctrl = MessageControl::new(extension);
        assert_eq!(
            info.header.extension.is_64bit(),
            ctrl.is_64bit(),
        );

        if !ctrl.is_64bit() { // 32-bit
            let casted = cap_header_acc.cast::<MsiCapability32Bit>();

            map_field!(casted.header).write(info.header);
            map_field!(casted.addr[0]).write(info.addr[0]);
            map_field!(casted.data).write(info.data);
        } else { // 64-bit
            let casted = cap_header_acc.cast::<MsiCapability64Bit>();

            map_field!(casted.header).write(info.header);
            map_field!(casted.addr[0]).write(info.addr[0]);
            map_field!(casted.addr[1]).write(info.addr[1]);
            map_field!(casted.data).write(info.data);
            // map_field!(casted.mask_bits).write(info.mask_bits);
            // map_field!(casted.pending_bits).write(info.pending_bits);

            // // or one-liner:
            // casted.write(info);
        }
    }

    /// Updates the MSI Capability info on given cap header.
    /// 
    /// Panics if the header is not MSI header.
    pub fn update_info<'a, M, F>(cap_header_acc: &impl AccessorTrait<'a, M, CapabilityHeader>, f: F)
    where
        M: DwordAccessMethod,
        F: FnOnce(Self) -> Self,
    {
        // use core::hint::black_box;

        // let updated = black_box(f(black_box(Self::read_info(cap_header_acc))));
        // black_box(Self::write_info(cap_header_acc, updated));

        Self::write_info(
            cap_header_acc,
            f(
                Self::read_info(cap_header_acc)
            )
        );
    }
}
