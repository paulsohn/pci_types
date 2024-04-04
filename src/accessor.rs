//! An accessor accesses to the PCI configuration space,
//! just like a pointer accesses to the main memory.

use core::mem::size_of;
use core::mem::MaybeUninit;
// use core::ptr::NonNull;
use core::marker::PhantomData;

use crate::address::{
    PciAddress,
    DwordAccessMethod,
};
use crate::access::{
    Access, Readable, Writable, ReadWrite, ReadOnly,
};

/// Default accessor implementation.
/// To use methods, refer to `AccessorTrait`.
pub struct DwordAccessor<'a, M, T, A = ReadWrite>
where
    M: DwordAccessMethod,
    A: Access,
{
    region: PciAddress,
    start_offset: u16,
    method: M,
    _access: PhantomData<A>,
    _marker: PhantomData<&'a T>,
}
impl<'a, M, T, A> AccessorTrait<'a, M, T, A> for DwordAccessor<'a, M, T, A>
where
    M: DwordAccessMethod,
    A: Access,
{
    type AccessorType<'a_, M_, T_, A_> = DwordAccessor<'a_, M_, T_, A_>
    where
        T_: 'a_,
        M_: DwordAccessMethod,
        A_: Access,
    ;

    fn new(region: PciAddress, start_offset: u16, method: M) -> Self {
        Self {
            region,
            start_offset,
            method,
            _access: PhantomData,
            _marker: PhantomData,
        }
    }

    fn region(&self) -> PciAddress {
        self.region
    }

    fn start_offset(&self) -> u16 {
        self.start_offset
    }

    fn method(&self) -> &M {
        &self.method
    }
}

// pub struct Wrapper<'a, M, T, ACC = DwordAccessor<'a, M, T>>
// where
//     M: DwordAccessMethod,
//     ACC: AccessorTrait<'a, M, T>,
// {
//     acc: ACC,
//     _marker: PhantomData<&'a T>,
//     _method: PhantomData<M>,
// }

pub trait AccessorTrait<'a, M, T, A = ReadWrite>
where
    M: DwordAccessMethod,
    A: Access,
{
    /// The concrete Accessor Type, which is intended to be `Self`-liked type.
    type AccessorType<'a_, M_, T_, A_>: AccessorTrait<'a_, M_, T_, A_>
    where
        T_: 'a_,
        M_: DwordAccessMethod,
        A_: Access,
    ;

    /// Creates a new accessor to the given type.
    fn new(region: PciAddress, start_offset: u16, method: M) -> Self;

    /// Extracts the region address from an accessor.
    fn region(&self) -> PciAddress;

    /// Extracts the start offset from an accessor.
    fn start_offset(&self) -> u16;

    /// Extracts the access method object from an accessor.
    fn method(&self) -> &M;

    /// Reads the value from the accessor.
    /// 
    /// The default implementation may be overriden;
    /// if a non-dword-based read is available, then `.method()` can be completely ignored.
    /// 
    /// In default implementation, the method panics if the reading range is not dword-aligned.
    fn read(&self) -> T
    where A: Readable
    {
        if size_of::<T>() < 4 { // sub-dword read
            let start_offset = self.start_offset();
            let end_offset = start_offset + (size_of::<T>() as u16) - 1; // inclusive end

            assert_eq!(
                start_offset / 4,
                end_offset / 4,
                "the value should be contained in a dword."
            );

            let start_pos = start_offset % 4;
            let end_pos = end_offset % 4;

            let region = self.region();
            let method: &M = self.method();

            unsafe {
                let raw = method.read_dword(
                    region,
                    start_offset - start_pos
                ).to_le_bytes();
                let slice = &raw[(start_pos as usize)..=(end_pos as usize)];

                let mut buf = MaybeUninit::<T>::uninit();
                MaybeUninit::copy_from_slice(buf.as_bytes_mut(), slice);

                buf.assume_init()
            }
        } else { // dword-aligned read
            assert!(self.start_offset() % 4 == 0, "the base should be 4-byte aligned.");
            assert!(size_of::<T>() % 4 == 0, "the type size should be multiple of 4.");

            let region = self.region();
            let start_offset = self.start_offset();
            let method: &M = self.method();

            unsafe {
                let mut buf = MaybeUninit::<T>::uninit();

                let (chunks, _) =  buf.as_bytes_mut().as_chunks_mut::<4>();
                for (i, chunk) in chunks.iter_mut().enumerate() {
                    let bytes = method.read_dword(
                        region,
                        start_offset + 4 * (i as u16)
                    ).to_le_bytes();

                    *chunk = MaybeUninit::new(bytes).transpose();
                    // MaybeUninit::write_slice(chunk, &bytes);
                }

                buf.assume_init()
            }
        }
    }

    /// Writes a value through the accessor.
    /// This requires `A` to be Readable also, in case of sub-dword writes.
    /// 
    /// The default implementation may be overriden;
    /// if a non-dword-based write is available, then `.method()` can be completely ignored.
    /// 
    /// In default implementation, the method panics if the writing range is not dword-aligned.
    fn write(&self, value: T)
    where A: Readable + Writable
    {
        if size_of::<T>() < 4 { // sub-dword write
            let start_offset = self.start_offset();
            let end_offset = start_offset + (size_of::<T>() as u16) - 1; // inclusive end

            assert_eq!(
                start_offset / 4,
                end_offset / 4,
                "the value should be contained in a dword."
            );

            let start_pos = start_offset % 4;
            let end_pos = end_offset % 4;

            let region = self.region();
            let method: &M = self.method();

            unsafe {
                let mut raw = method.read_dword(
                    region,
                    start_offset - start_pos
                ).to_le_bytes();
                let dword_slice = &mut raw[(start_pos as usize)..=(end_pos as usize)];

                let src = MaybeUninit::new(value);
                let src_slice = MaybeUninit::slice_assume_init_ref(
                    src.as_bytes()
                );
                dword_slice.copy_from_slice(src_slice);

                method.write_dword(
                    region,
                    start_offset - start_pos,
                    u32::from_le_bytes(raw)
                );
            }
        } else { // dword-aligned write
            assert!(self.start_offset() % 4 == 0, "offset should be 4-byte aligned.");
            assert!(size_of::<T>() % 4 == 0, "The type size should be multiple of 4.");

            let region = self.region();
            let start_offset = self.start_offset();
            let method: &M = self.method();

            unsafe {
                let buf = MaybeUninit::new(value);

                let (chunks, _) = buf.as_bytes().as_chunks::<4>();
                for (i, chunk) in chunks.iter().enumerate() {
                    let value = u32::from_le_bytes(
                        MaybeUninit::array_assume_init(*chunk)
                    );

                    method.write_dword(
                        region,
                        start_offset + 4 * (i as u16),
                        value
                    );
                }
            }
        }
    }

    fn update<F>(&self, f: F)
    where
        A: Readable + Writable,
        F: FnOnce(T) -> T
    {
        self.write(f(self.read()))
    }

    /// Constructs a new accessor by mapping the wrapped pointer.
    /// 
    /// **Note** : The offset is passed and returned in pointer type
    /// because the offset arithmetic followes the very same rule of that of the pointer;
    /// but this is only for the sake of convenience,
    /// which lets compiler to infer the type and thus the field offsets.
    /// The actual pointer value should be the offset in the config region.
    /// 
    /// If you are unsure, use `map_field` instead of this function.
    unsafe fn map<U, F>(&self, f: F) -> Self::AccessorType<'a, M, U, A>
    where
        F: FnOnce(*mut T) -> *mut U,
    {
        Self::AccessorType::new(
            self.region(),
            f(self.start_offset() as usize as *mut _) as usize as u16,
            self.method().clone()
        )
    }

    /// Constructs a new accessor of the same type, but of different offset.
    fn with_offset(&self, new_offset: u16) -> Self
    where Self: Sized
    {
        Self::new(
            self.region(),
            new_offset,
            self.method().clone()
        )
    }

    /// Cast the accessor into an accessor of different type.
    fn cast<U>(&self) -> Self::AccessorType<'a, M, U, A> {
        unsafe { self.map(|ptr| ptr as *mut U) }
    }

    fn read_only(self) -> Self::AccessorType<'a, M, T, ReadOnly>
    where
        Self: Sized,
        A: Readable + Writable,
    {
        Self::AccessorType::new(
            self.region(),
            self.start_offset(),
            self.method().clone()
        )
    }
}

/// the copy-paste of
/// https://github.com/rust-osdev/volatile/blob/main/src/volatile_ptr/macros.rs
#[macro_export]
macro_rules! map_field {
    ($accessor:ident $(.$place:ident)*$([$idx:expr])?) => {{
        unsafe {
            $accessor.map(|ptr| {
                core::ptr::addr_of_mut!((*ptr)$(.$place)*$([$idx])? )
            })
        }
    }};
}