use core::marker::PhantomData;
// use core::fmt::{Debug, Formatter};

use crate::{DwordAccessMethod, DwordAccessor, AccessorTrait};

pub mod msi;

// #[derive(Clone)]
// pub struct PciCapabilityAddress {
//     pub address: PciAddress,
//     pub offset: u16,
// }

// impl Debug for PciCapabilityAddress {
//     fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
//         write!(f, "{}, offset: {:02x}", self.address, self.offset)
//     }
// }

// /// PCI capabilities
// #[derive(Clone, Debug)]
// pub enum PciCapability {
//     /// Power management capability, Cap ID = `0x01`
//     PowerManagement(PciCapabilityAddress),
//     /// Accelerated graphics port capability, Cap ID = `0x02`
//     AcceleratedGraphicsPort(PciCapabilityAddress),
//     /// Vital product data capability, Cap ID = `0x3`
//     VitalProductData(PciCapabilityAddress),
//     /// Slot identification capability, Cap ID = `0x04`
//     SlotIdentification(PciCapabilityAddress),
//     /// Message signalling interrupts capability, Cap ID = `0x05`
//     Msi(MsiCapability),
//     /// CompactPCI HotSwap capability, Cap ID = `0x06`
//     CompactPCIHotswap(PciCapabilityAddress),
//     /// PCI-X capability, Cap ID = `0x07`
//     PciX(PciCapabilityAddress),
//     /// HyperTransport capability, Cap ID = `0x08`
//     HyperTransport(PciCapabilityAddress),
//     /// Vendor-specific capability, Cap ID = `0x09`
//     Vendor(PciCapabilityAddress),
//     /// Debug port capability, Cap ID = `0x0A`
//     DebugPort(PciCapabilityAddress),
//     /// CompactPCI Central Resource Control capability, Cap ID = `0x0B`
//     CompactPCICentralResourceControl(PciCapabilityAddress),
//     /// PCI Standard Hot-Plug Controller capability, Cap ID = `0x0C`
//     PciHotPlugControl(PciCapabilityAddress),
//     /// Bridge subsystem vendor/device ID capability, Cap ID = `0x0D`
//     BridgeSubsystemVendorId(PciCapabilityAddress),
//     /// AGP Target PCI-PCI bridge capability, Cap ID = `0x0E`
//     AGP3(PciCapabilityAddress),
//     /// PCI Express capability, Cap ID = `0x10`
//     PciExpress(PciCapabilityAddress),
//     /// MSI-X capability, Cap ID = `0x11`
//     MsiX(PciCapabilityAddress),
//     /// Unknown capability
//     Unknown {
//         address: PciCapabilityAddress,
//         id: u8,
//     },
// }

// impl PciCapability {
//     fn parse(id: u8, address: PciCapabilityAddress, extension: u16) -> Option<PciCapability> {
//         match id {
//             0x00 => None, // null capability
//             0x01 => Some(PciCapability::PowerManagement(address)),
//             0x02 => Some(PciCapability::AcceleratedGraphicsPort(address)),
//             0x03 => Some(PciCapability::VitalProductData(address)),
//             0x04 => Some(PciCapability::SlotIdentification(address)),
//             0x05 => Some(PciCapability::Msi(MsiCapability::new(address, extension))),
//             0x06 => Some(PciCapability::CompactPCIHotswap(address)),
//             0x07 => Some(PciCapability::PciX(address)),
//             0x08 => Some(PciCapability::HyperTransport(address)),
//             0x09 => Some(PciCapability::Vendor(address)),
//             0x0A => Some(PciCapability::DebugPort(address)),
//             0x0B => Some(PciCapability::CompactPCICentralResourceControl(address)),
//             0x0C => Some(PciCapability::PciHotPlugControl(address)),
//             0x0D => Some(PciCapability::BridgeSubsystemVendorId(address)),
//             0x0E => Some(PciCapability::AGP3(address)),
//             0x10 => Some(PciCapability::PciExpress(address)),
//             0x11 => Some(PciCapability::MsiX(address)),
//             _ => Some(PciCapability::Unknown { address, id }),
//         }
//     }
// }

// todo: make CapabilityID enum.



#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CapabilityHeader<EXT = u16> {
    /// Capability ID
    pub id: u8,
    /// Next capability offset in 8 bits
    pub next: u8,
    /// 16-bit extension. The exact semantics is capability type-dependent.
    pub extension: EXT, // todo: ensure 16-bit
}

pub struct CapabilityIterator<'a, M, ACC = DwordAccessor<'a, M, CapabilityHeader>>
where
    M: DwordAccessMethod,
    ACC: AccessorTrait<'a, M, CapabilityHeader>,
{
    // this should be a concrete type wrapping an accessor,
    // in order to bound generic var `M` in capabilities type signature.
    acc: ACC,
    _marker: PhantomData<&'a CapabilityHeader>,
    _method: PhantomData<M>,
}
impl<'a, M, ACC> CapabilityIterator<'a, M, ACC>
where
    M: DwordAccessMethod,
    ACC: AccessorTrait<'a, M, CapabilityHeader>,
{
    pub fn new(acc: ACC) -> Self {
        Self {
            acc,
            _marker: PhantomData,
            _method: PhantomData,
        }
    }
}

// use crate::accessor::Wrapper;
// pub type CapabilityIteratorAsWrapper<'a, M, ACC = DwordAccessor<'a, M, CapabilityHeader>> = Wrapper<'a, M, CapabilityHeader, ACC>;

impl<'a, M, ACC> Iterator for CapabilityIterator<'a, M, ACC>
where
    M: DwordAccessMethod,
    ACC: AccessorTrait<'a, M, CapabilityHeader>,
{
    type Item = ACC;

    fn next(&mut self) -> Option<Self::Item> {
        if self.acc.start_offset() == 0 {
            return None;
        }
        let CapabilityHeader { next, .. } = self.acc.read();

        Some(self.acc.with_offset(next as u16))
    }
}