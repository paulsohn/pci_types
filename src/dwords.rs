//! Registers of dword-sized or smaller sizes.

use core::convert::TryFrom;
use core::fmt::{Formatter, Debug};
use bit_field::BitField;
use crate::headers::HeaderType;
use crate::device_type::DeviceType;

/// (vendor id, id) register for device or subsystems.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IDs {
    /// The vendor id of this device or subsystem.
    pub vendor_id: u16,
    /// The device id, or subsystem id if it is a subsystem register.
    pub id: u16,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CmdSts {
    cmd: Command,
    sts: Status,
}
impl CmdSts {
    pub fn status(&self) -> Status {
        self.sts
    }

    pub fn command(&self) -> Command {
        self.cmd
    }

    /// Set command register.
    /// Writing the dword after calling this will reset all RW1C bits in status register.
    /// 
    /// Use this if all status bits are acknowledged and handled.
    /// Otherwise, use `.set_command_preserving_status(f)`.
    pub fn set_command<F>(&mut self, f: F)
    where
        F: FnOnce(Command) -> Command,
    {
        self.cmd = f(self.cmd);
    }

    /// Set command register.
    /// Writing the dword after calling this will preserve all RW1C bits in status register.
    pub fn set_command_preserving_status<F>(&mut self, f: F)
    where
        F: FnOnce(Command) -> Command,
    {
        self.cmd = f(self.cmd);
        self.sts = Status::zero();
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Command(u16);
impl Command {
    pub fn io_enabled(&self) -> bool {
        self.0.get_bit(0)
    }
    pub fn set_io_enabled(&mut self) {
        self.0.set_bit(0, true);
    }
    pub fn clear_io_enabled(&mut self) {
        self.0.set_bit(0, false);
    }

    pub fn memory_enabled(&self) -> bool {
        self.0.get_bit(1)
    }
    pub fn set_memory_enabled(&mut self) {
        self.0.set_bit(1, true);
    }
    pub fn clear_memory_enabled(&mut self) {
        self.0.set_bit(1, false);
    }

    pub fn bus_master_enabled(&self) -> bool {
        self.0.get_bit(2)
    }
    pub fn set_bus_master_enabled(&mut self) {
        self.0.set_bit(2, true);
    }
    pub fn clear_bus_master_enabled(&mut self) {
        self.0.set_bit(2, false);
    }

    pub fn special_cycle_enabled(&self) -> bool {
        self.0.get_bit(3)
    }
    // pub fn set_special_cycle_enabled(&mut self) {
    //     self.0.set_bit(3, true);
    // }
    // pub fn clear_special_cycle_enabled(&mut self) {
    //     self.0.set_bit(3, false);
    // }

    pub fn memory_write_and_invalidate_enabled(&self) -> bool {
        self.0.get_bit(4)
    }

    pub fn vga_palette_snoop(&self) -> bool {
        self.0.get_bit(5)
    }

    pub fn parity_error_response(&self) -> bool {
        self.0.get_bit(6)
    }
    pub fn set_parity_error_response(&mut self) {
        self.0.set_bit(6, true);
    }
    pub fn clear_parity_error_response(&mut self) {
        self.0.set_bit(6, false);
    }

    pub fn idsel_step_wait_cycle_control(&self) -> bool {
        self.0.get_bit(7)
    }

    pub fn serr_enabled(&self) -> bool {
        self.0.get_bit(8)
    }
    pub fn set_serr_enabled(&mut self) {
        self.0.set_bit(8, true);
    }
    pub fn clear_serr_enabled(&mut self) {
        self.0.set_bit(8, false);
    }

    pub fn fast_back_to_back_enabled(&self) -> bool {
        self.0.get_bit(9)
    }

    pub fn interrupt_disabled(&self) -> bool {
        self.0.get_bit(10)
    }
    pub fn set_interrupt_disabled(&mut self) {
        self.0.set_bit(10, true);
    }
    pub fn clear_interrupt_disabled(&mut self) {
        self.0.set_bit(10, false);
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Status(u16);
impl Status {
    /// Make a zeroed status register.
    /// All bits in status register are RW1C, so you should write this on command register write.
    pub(crate) fn zero() -> Self {
        Self(0)
    }

    /// Represents the state of the device's INTx# signal. If returns `true` and bit 10 of the
    /// Command register (Interrupt Disable bit) is set to 0 the signal will be asserted;
    /// otherwise, the signal will be ignored.
    /// 
    /// In secondary status, this is `false`.
    pub fn interrupt_status(&self) -> bool {
        self.0.get_bit(3)
    }

    /// If returns `true` the device implements the pointer for a New Capabilities Linked list;
    /// otherwise, the linked list is not available.
    ///
    /// For PCIe always set to `true`.
    /// 
    /// In secondary status, this is `false`.
    pub fn has_capability_list(&self) -> bool {
        self.0.get_bit(4)
    }

    /// If returns `true` the device is capable of running at 66 MHz; otherwise, the device runs at 33 MHz.
    ///
    /// For PCIe always set to `false`
    pub fn capable_66mhz(&self) -> bool {
        self.0.get_bit(5)
    }

    /// If returns `true` the device can accept fast back-to-back transactions that are not from
    /// the same agent; otherwise, transactions can only be accepted from the same agent.
    ///
    /// For PCIe always set to `false`
    pub fn fast_back_to_back_capable(&self) -> bool {
        self.0.get_bit(7)
    }

    /// This returns `true` only when the following conditions are met:
    /// - The bus agent asserted PERR# on a read or observed an assertion of PERR# on a write
    /// - the agent setting the bit acted as the bus master for the operation in which the error occurred
    /// - bit 6 of the Command register (Parity Error Response bit) is set to 1.
    pub fn master_data_parity_error(&self) -> bool {
        self.0.get_bit(8)
    }

    /// The slowest time that a device will assert DEVSEL# for any bus command except
    /// Configuration Space read and writes.
    ///
    /// For PCIe always set to `Fast`
    pub fn devsel_timing(&self) -> Option<DevselTiming> {
        let bits = self.0.get_bits(9..=10);
        DevselTiming::try_from(bits as u8).ok()
    }

    /// Will return `true` whenever a target device terminates a transaction with Target-Abort.
    pub fn signalled_target_abort(&self) -> bool {
        self.0.get_bit(11)
    }

    /// Will return `true`, by a master device, whenever its transaction is terminated with Target-Abort.
    pub fn received_target_abort(&self) -> bool {
        self.0.get_bit(12)
    }

    /// Will return `true`, by a master device, whenever its transaction
    /// (except for Special Cycle transactions) is terminated with Master-Abort.
    pub fn received_master_abort(&self) -> bool {
        self.0.get_bit(13)
    }

    /// Will be `true` whenever the device asserts SERR#.
    pub fn signalled_system_error(&self) -> bool {
        self.0.get_bit(14)
    }

    /// Will be `true` whenever the device detects a parity error, even if parity error handling is disabled.
    pub fn parity_error_detected(&self) -> bool {
        self.0.get_bit(15)
    }
}
impl Debug for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StatusRegister")
            .field("parity_error_detected", &self.parity_error_detected())
            .field("signalled_system_error", &self.signalled_system_error())
            .field("received_master_abort", &self.received_master_abort())
            .field("received_target_abort", &self.received_target_abort())
            .field("signalled_target_abort", &self.signalled_target_abort())
            .field("devsel_timing", &self.devsel_timing())
            .field("master_data_parity_error", &self.master_data_parity_error())
            .field("fast_back_to_back_capable", &self.fast_back_to_back_capable())
            .field("capable_66mhz", &self.capable_66mhz())
            .field("has_capability_list", &self.has_capability_list())
            .field("interrupt_status", &self.interrupt_status())
            .finish()
    }
}

/// Slowest time that a device will assert DEVSEL# for any bus command except Configuration Space
/// read and writes
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevselTiming {
    Fast = 0x0,
    Medium = 0x1,
    Slow = 0x2,
}
impl TryFrom<u8> for DevselTiming {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(DevselTiming::Fast),
            0x1 => Ok(DevselTiming::Medium),
            0x2 => Ok(DevselTiming::Slow),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RevCC {
    pub revision_id: u8,
    pub interface: u8,
    pub sub_class: u8,
    pub base_class: u8,
}
impl RevCC {
    pub fn device_type(&self) -> DeviceType {
        DeviceType::from((self.base_class, self.sub_class))
    }
}

/// The miscellaneous collection of byte-size registers
/// which are in the same dword with the Header Type register.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeaderTypeDword {
    pub cacheline_size: u8,
    pub latency_timer: u8,
    header_type_raw: u8,
    pub bist: u8,
}
impl HeaderTypeDword {
    pub fn header_type(&self) -> Result<HeaderType, u8> {
        let ty = match self.header_type_raw.get_bits(0..=6) {
            0x00 => HeaderType::Endpoint,
            0x01 => HeaderType::PciPciBridge,
            0x02 => HeaderType::CardBusBridge,
            t => { return Err(t); },
        };
        Ok(ty)
    }
    pub fn has_multiple_functions(&self) -> bool {
        self.header_type_raw.get_bit(7)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BusNos {
    /// Primary PCI (in PCI-PCI Bridge Header) or PCI (in PCI-Cardbus Bridge Header) bus number.
    pub primary_bus_number: u8,
    /// Secondary PCI (in PCI-PCI Bridge Header) or Cardbus (in PCI-Cardbus Bridge Header) bus number.
    pub secondary_bus_number: u8,
    /// Subordinate bus number.
    pub subordinate_bus_number: u8,
    /// Secondary PCI (in PCI-PCI Bridge Header) or Cardbus (in PCI-Cardbus Bridge Header) latency timer.
    pub secondary_latency_timer: u8,
}

/// Interrupt Pin and Line registers.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Intr {
    pub interrupt_pin: u8,
    pub interrupt_line: u8,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EPLimits {
    pub min_grant: u8,
    pub max_latency: u8,
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BridgeControl(pub u16);