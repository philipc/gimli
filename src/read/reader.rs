use borrow::Cow;
use std::fmt::Debug;

use common::{Format, Offset};
use endianity::Endianity;
use leb128;
use read::{Error, Result};

/// The type used for offsets and lengths in the `Reader` trait.
pub type ReaderOffset = Offset;

/// A trait for reading the data from a DWARF section.
///
/// All read operations advance the section offset of the reader
/// unless specified otherwise.
///
/// ## Choosing a `Reader` Implementation
///
/// `gimli` comes with a few different `Reader` implementations and lets you
/// choose the one that is right for your use case. A `Reader` is essentially a
/// view into the raw bytes that make up some DWARF, but this view might borrow
/// the underlying data or use reference counting ownership, and it might be
/// thread safe or not.
///
/// | Implementation    | Ownership         | Thread Safe | Notes |
/// |:------------------|:------------------|:------------|:------|
/// | [`EndianSlice`](./struct.EndianSlice.html)        | Borrowed          | Yes         | Fastest, but requires that all of your code work with borrows. |
/// | [`EndianRcSlice`](./struct.EndianRcSlice.html)    | Reference counted | No          | Shared ownership via reference counting, which alleviates the borrow restrictions of `EndianSlice` but imposes reference counting increments and decrements. Cannot be sent across threads, because the reference count is not atomic. |
/// | [`EndianArcSlice`](./struct.EndianArcSlice.html)  | Reference counted | Yes         | The same as `EndianRcSlice`, but uses atomic reference counting, and therefore reference counting operations are slower but `EndianArcSlice`s may be sent across threads. |
/// | [`EndianReader<T>`](./struct.EndianReader.html)   | Same as `T`       | Same as `T` | Escape hatch for easily defining your own type of `Reader`. |
pub trait Reader: Debug + Clone {
    /// The endianity of bytes that are read.
    type Endian: Endianity;

    /// Return the endianity of bytes that are read.
    fn endian(&self) -> Self::Endian;

    /// Return the number of bytes remaining.
    fn len(&self) -> ReaderOffset;

    /// Set the number of bytes remaining to zero.
    fn empty(&mut self);

    /// Set the number of bytes remaining to the specified length.
    fn truncate(&mut self, len: ReaderOffset) -> Result<()>;

    /// Return the offset of this reader's data relative to the start of
    /// the given base reader's data.
    ///
    /// May panic if this reader's data is not contained within the given
    /// base reader's data.
    fn offset_from(&self, base: &Self) -> ReaderOffset;

    /// Find the index of the first occurence of the given byte.
    /// The offset of the reader is not changed.
    fn find(&self, byte: u8) -> Result<ReaderOffset>;

    /// Discard the specified number of bytes.
    fn skip(&mut self, len: ReaderOffset) -> Result<()>;

    /// Split a reader in two.
    ///
    /// A new reader is returned that can be used to read the next
    /// `len` bytes, and `self` is advanced so that it reads the remainder.
    fn split(&mut self, len: ReaderOffset) -> Result<Self>;

    /// Return all remaining data as a clone-on-write slice.
    ///
    /// The slice will be borrowed where possible, but some readers may
    /// always return an owned vector.
    ///
    /// Does not advance the reader.
    fn to_slice(&self) -> Result<Cow<[u8]>>;

    /// Convert all remaining data to a clone-on-write string.
    ///
    /// The string will be borrowed where possible, but some readers may
    /// always return an owned string.
    ///
    /// Does not advance the reader.
    ///
    /// Returns an error if the data contains invalid characters.
    fn to_string(&self) -> Result<Cow<str>>;

    /// Convert all remaining data to a clone-on-write string, including invalid characters.
    ///
    /// The string will be borrowed where possible, but some readers may
    /// always return an owned string.
    ///
    /// Does not advance the reader.
    fn to_string_lossy(&self) -> Result<Cow<str>>;

    /// Read a u8 array.
    fn read_u8_array<A>(&mut self) -> Result<A>
    where
        A: Sized + Default + AsMut<[u8]>;

    /// Return true if the number of bytes remaining is zero.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read a u8.
    #[inline]
    fn read_u8(&mut self) -> Result<u8> {
        let a: [u8; 1] = self.read_u8_array()?;
        Ok(a[0])
    }

    /// Read an i8.
    #[inline]
    fn read_i8(&mut self) -> Result<i8> {
        let a: [u8; 1] = self.read_u8_array()?;
        Ok(a[0] as i8)
    }

    /// Read a u16.
    #[inline]
    fn read_u16(&mut self) -> Result<u16> {
        let a: [u8; 2] = self.read_u8_array()?;
        Ok(self.endian().read_u16(&a))
    }

    /// Read an i16.
    #[inline]
    fn read_i16(&mut self) -> Result<i16> {
        let a: [u8; 2] = self.read_u8_array()?;
        Ok(self.endian().read_i16(&a))
    }

    /// Read a u32.
    #[inline]
    fn read_u32(&mut self) -> Result<u32> {
        let a: [u8; 4] = self.read_u8_array()?;
        Ok(self.endian().read_u32(&a))
    }

    /// Read an i32.
    #[inline]
    fn read_i32(&mut self) -> Result<i32> {
        let a: [u8; 4] = self.read_u8_array()?;
        Ok(self.endian().read_i32(&a))
    }

    /// Read a u64.
    #[inline]
    fn read_u64(&mut self) -> Result<u64> {
        let a: [u8; 8] = self.read_u8_array()?;
        Ok(self.endian().read_u64(&a))
    }

    /// Read an i64.
    #[inline]
    fn read_i64(&mut self) -> Result<i64> {
        let a: [u8; 8] = self.read_u8_array()?;
        Ok(self.endian().read_i64(&a))
    }

    /// Read a f32.
    #[inline]
    fn read_f32(&mut self) -> Result<f32> {
        let a: [u8; 4] = self.read_u8_array()?;
        Ok(self.endian().read_f32(&a))
    }

    /// Read a f64.
    #[inline]
    fn read_f64(&mut self) -> Result<f64> {
        let a: [u8; 8] = self.read_u8_array()?;
        Ok(self.endian().read_f64(&a))
    }

    /// Read a null-terminated slice, and return it (excluding the null).
    fn read_null_terminated_slice(&mut self) -> Result<Self> {
        let idx = self.find(0)?;
        let val = self.split(idx)?;
        self.skip(1)?;
        Ok(val)
    }

    /// Read an unsigned LEB128 encoded integer.
    fn read_uleb128(&mut self) -> Result<u64> {
        leb128::read::unsigned(self)
    }

    /// Read a signed LEB128 encoded integer.
    fn read_sleb128(&mut self) -> Result<i64> {
        leb128::read::signed(self)
    }

    /// Read an initial length field.
    ///
    /// This field is encoded as either a 32-bit length or
    /// a 64-bit length, and the returned `Format` indicates which.
    fn read_initial_length(&mut self) -> Result<(ReaderOffset, Format)> {
        const MAX_DWARF_32_UNIT_LENGTH: u32 = 0xffff_fff0;
        const DWARF_64_INITIAL_UNIT_LENGTH: u32 = 0xffff_ffff;

        let val = self.read_u32()?;
        if val < MAX_DWARF_32_UNIT_LENGTH {
            Ok((ReaderOffset::from(val), Format::Dwarf32))
        } else if val == DWARF_64_INITIAL_UNIT_LENGTH {
            let val = self.read_u64()?;
            Ok((val, Format::Dwarf64))
        } else {
            Err(Error::UnknownReservedLength)
        }
    }

    /// Read an address-sized integer, and return it as a `u64`.
    fn read_address(&mut self, address_size: u8) -> Result<u64> {
        match address_size {
            1 => self.read_u8().map(u64::from),
            2 => self.read_u16().map(u64::from),
            4 => self.read_u32().map(u64::from),
            8 => self.read_u64(),
            otherwise => Err(Error::UnsupportedAddressSize(otherwise)),
        }
    }

    /// Parse a word-sized integer according to the DWARF format.
    ///
    /// These are always used to encode section offsets or lengths,
    /// and so have a type of `ReaderOffset`.
    fn read_word(&mut self, format: Format) -> Result<ReaderOffset> {
        match format {
            Format::Dwarf32 => self.read_u32().map(ReaderOffset::from),
            Format::Dwarf64 => self.read_u64(),
        }
    }

    /// Parse a word-sized section length according to the DWARF format.
    #[inline]
    fn read_length(&mut self, format: Format) -> Result<ReaderOffset> {
        self.read_word(format)
    }

    /// Parse a word-sized section offset according to the DWARF format.
    #[inline]
    fn read_offset(&mut self, format: Format) -> Result<ReaderOffset> {
        self.read_word(format)
    }

    /// Parse a section offset of the given size.
    ///
    /// This is used for `DW_FORM_ref_addr` values in DWARF version 2.
    fn read_sized_offset(&mut self, size: u8) -> Result<ReaderOffset> {
        match size {
            1 => self.read_u8().map(u64::from),
            2 => self.read_u16().map(u64::from),
            4 => self.read_u32().map(u64::from),
            8 => self.read_u64(),
            otherwise => Err(Error::UnsupportedOffsetSize(otherwise)),
        }
    }
}
