//! Read DWARF debugging information.

use std::fmt::{self, Debug};
use std::result;
#[cfg(feature = "std")]
use std::{error, io};

use common::Register;
use constants;

mod cfi;
pub use self::cfi::*;

mod endian_slice;
pub use self::endian_slice::*;

mod endian_reader;
pub use self::endian_reader::*;

mod reader;
pub use self::reader::*;

mod abbrev;
pub use self::abbrev::*;

mod aranges;
pub use self::aranges::*;

mod line;
pub use self::line::*;

mod loclists;
pub use self::loclists::*;

mod lookup;

mod op;
pub use self::op::*;

mod pubnames;
pub use self::pubnames::*;

mod pubtypes;
pub use self::pubtypes::*;

mod rnglists;
pub use self::rnglists::*;

mod str;
pub use self::str::*;

mod unit;
pub use self::unit::*;

mod value;
pub use self::value::*;

/// `EndianBuf` has been renamed to `EndianSlice`. For ease of upgrading across
/// `gimli` versions, we export this type alias.
#[deprecated(note = "EndianBuf has been renamed to EndianSlice, use that instead.")]
pub type EndianBuf<'input, Endian> = EndianSlice<'input, Endian>;

/// An error that occurred when parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An I/O error occurred while reading.
    Io,
    /// Found a PC relative pointer, but the section base is undefined.
    PcRelativePointerButSectionBaseIsUndefined,
    /// Found a `.text` relative pointer, but the `.text` base is undefined.
    TextRelativePointerButTextBaseIsUndefined,
    /// Found a data relative pointer, but the data base is undefined.
    DataRelativePointerButDataBaseIsUndefined,
    /// Found a function relative pointer in a context that does not have a
    /// function base.
    FuncRelativePointerInBadContext,
    /// An error parsing an unsigned LEB128 value.
    BadUnsignedLeb128,
    /// An error parsing a signed LEB128 value.
    BadSignedLeb128,
    /// An abbreviation declared that its tag is zero, but zero is reserved for
    /// null records.
    AbbreviationTagZero,
    /// An attribute specification declared that its form is zero, but zero is
    /// reserved for null records.
    AttributeFormZero,
    /// The abbreviation's has-children byte was not one of
    /// `DW_CHILDREN_{yes,no}`.
    BadHasChildren,
    /// The specified length is impossible.
    BadLength,
    /// Found an unknown `DW_FORM_*` type.
    UnknownForm,
    /// Expected a zero, found something else.
    ExpectedZero,
    /// Found an abbreviation code that has already been used.
    DuplicateAbbreviationCode,
    /// Found a duplicate arange.
    DuplicateArange,
    /// Found an unknown reserved length value.
    UnknownReservedLength,
    /// Found an unknown DWARF version.
    UnknownVersion(u64),
    /// Found a record with an unknown abbreviation code.
    UnknownAbbreviation,
    /// Hit the end of input before it was expected.
    UnexpectedEof,
    /// Read a null entry before it was expected.
    UnexpectedNull,
    /// Found an unknown standard opcode.
    UnknownStandardOpcode(constants::DwLns),
    /// Found an unknown extended opcode.
    UnknownExtendedOpcode(constants::DwLne),
    /// The specified address size is not supported.
    UnsupportedAddressSize(u8),
    /// The specified offset size is not supported.
    UnsupportedOffsetSize(u8),
    /// The specified field size is not supported.
    UnsupportedFieldSize(u8),
    /// The minimum instruction length must not be zero.
    MinimumInstructionLengthZero,
    /// The maximum operations per instruction must not be zero.
    MaximumOperationsPerInstructionZero,
    /// The line range must not be zero.
    LineRangeZero,
    /// The opcode base must not be zero.
    OpcodeBaseZero,
    /// Found an invalid UTF-8 string.
    BadUtf8,
    /// Expected to find the CIE ID, but found something else.
    NotCieId,
    /// Expected to find a pointer to a CIE, but found the CIE ID instead.
    NotCiePointer,
    /// Expected to find a pointer to an FDE, but found a CIE instead.
    NotFdePointer,
    /// Invalid branch target for a DW_OP_bra or DW_OP_skip.
    BadBranchTarget(u64),
    /// DW_OP_push_object_address used but no address passed in.
    InvalidPushObjectAddress,
    /// Not enough items on the stack when evaluating an expression.
    NotEnoughStackItems,
    /// Too many iterations to compute the expression.
    TooManyIterations,
    /// An unrecognized operation was found while parsing a DWARF
    /// expression.
    InvalidExpression(constants::DwOp),
    /// The expression had a piece followed by an expression
    /// terminator without a piece.
    InvalidPiece,
    /// An expression-terminating operation was followed by something
    /// other than the end of the expression or a piece operation.
    InvalidExpressionTerminator(u64),
    /// Division or modulus by zero when evaluating an expression.
    DivisionByZero,
    /// An expression operation used mismatching types.
    TypeMismatch,
    /// An expression operation required an integral type but saw a
    /// floating point type.
    IntegralTypeRequired,
    /// An expression operation used types that are not supported.
    UnsupportedTypeOperation,
    /// The shift value in an expression must be a non-negative integer.
    InvalidShiftExpression,
    /// An unknown DW_CFA_* instruction.
    UnknownCallFrameInstruction(constants::DwCfa),
    /// The end of an address range was before the beginning.
    InvalidAddressRange,
    /// The end offset of a loc list entry was before the beginning.
    InvalidLocationAddressRange,
    /// Encountered a call frame instruction in a context in which it is not
    /// valid.
    CfiInstructionInInvalidContext,
    /// When evaluating call frame instructions, found a `DW_CFA_restore_state`
    /// stack pop instruction, but the stack was empty, and had nothing to pop.
    PopWithEmptyStack,
    /// Do not have unwind info for the given address.
    NoUnwindInfoForAddress,
    /// An offset value was larger than the maximum supported value.
    UnsupportedOffset,
    /// The given pointer encoding is either unknown or invalid.
    UnknownPointerEncoding,
    /// Did not find an entry at the given offset.
    NoEntryAtGivenOffset,
    /// The given offset is out of bounds.
    OffsetOutOfBounds,
    /// Found an unknown CFI augmentation.
    UnknownAugmentation,
    /// We do not support the given pointer encoding yet.
    UnsupportedPointerEncoding,
    /// Registers larger than `u16` are not supported.
    UnsupportedRegister(u64),
    /// The CFI program defined more register rules than we have storage for.
    TooManyRegisterRules,
    /// Attempted to push onto the CFI stack, but it was already at full
    /// capacity.
    CfiStackFull,
    /// The `.eh_frame_hdr` binary search table claims to be variable-length encoded,
    /// which makes binary search impossible.
    VariableLengthSearchTable,
    /// The `DW_UT_*` value for this unit is not supported yet.
    UnsupportedUnitType,
    /// Ranges using AddressIndex are not supported yet.
    UnsupportedAddressIndex,
    /// Nonzero segment selector sizes aren't supported yet.
    UnsupportedSegmentSize,
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        Debug::fmt(self, f)
    }
}

impl Error {
    /// A short description of the error.
    pub fn description(&self) -> &str {
        match *self {
            Error::Io => "An I/O error occurred while reading.",
            Error::PcRelativePointerButSectionBaseIsUndefined => {
                "Found a PC relative pointer, but the section base is undefined."
            }
            Error::TextRelativePointerButTextBaseIsUndefined => {
                "Found a `.text` relative pointer, but the `.text` base is undefined."
            }
            Error::DataRelativePointerButDataBaseIsUndefined => {
                "Found a data relative pointer, but the data base is undefined."
            }
            Error::FuncRelativePointerInBadContext => {
                "Found a function relative pointer in a context that does not have a function base."
            }
            Error::BadUnsignedLeb128 => "An error parsing an unsigned LEB128 value",
            Error::BadSignedLeb128 => "An error parsing a signed LEB128 value",
            Error::AbbreviationTagZero => {
                "An abbreviation declared that its tag is zero,
                 but zero is reserved for null records"
            }
            Error::AttributeFormZero => {
                "An attribute specification declared that its form is zero,
                 but zero is reserved for null records"
            }
            Error::BadHasChildren => {
                "The abbreviation's has-children byte was not one of
                 `DW_CHILDREN_{yes,no}`"
            }
            Error::BadLength => "The specified length is impossible",
            Error::UnknownForm => "Found an unknown `DW_FORM_*` type",
            Error::ExpectedZero => "Expected a zero, found something else",
            Error::DuplicateAbbreviationCode => {
                "Found an abbreviation code that has already been used"
            }
            Error::DuplicateArange => "Found a duplicate arange",
            Error::UnknownReservedLength => "Found an unknown reserved length value",
            Error::UnknownVersion(_) => "Found an unknown DWARF version",
            Error::UnknownAbbreviation => "Found a record with an unknown abbreviation code",
            Error::UnexpectedEof => "Hit the end of input before it was expected",
            Error::UnexpectedNull => "Read a null entry before it was expected.",
            Error::UnknownStandardOpcode(_) => "Found an unknown standard opcode",
            Error::UnknownExtendedOpcode(_) => "Found an unknown extended opcode",
            Error::UnsupportedAddressSize(_) => "The specified address size is not supported",
            Error::UnsupportedOffsetSize(_) => "The specified offset size is not supported",
            Error::UnsupportedFieldSize(_) => "The specified field size is not supported",
            Error::MinimumInstructionLengthZero => {
                "The minimum instruction length must not be zero."
            }
            Error::MaximumOperationsPerInstructionZero => {
                "The maximum operations per instruction must not be zero."
            }
            Error::LineRangeZero => "The line range must not be zero.",
            Error::OpcodeBaseZero => "The opcode base must not be zero.",
            Error::BadUtf8 => "Found an invalid UTF-8 string.",
            Error::NotCieId => "Expected to find the CIE ID, but found something else.",
            Error::NotCiePointer => "Expected to find a CIE pointer, but found the CIE ID instead.",
            Error::NotFdePointer => {
                "Expected to find an FDE pointer, but found a CIE pointer instead."
            }
            Error::BadBranchTarget(_) => "Invalid branch target in DWARF expression",
            Error::InvalidPushObjectAddress => {
                "DW_OP_push_object_address used but no object address given"
            }
            Error::NotEnoughStackItems => "Not enough items on stack when evaluating expression",
            Error::TooManyIterations => "Too many iterations to evaluate DWARF expression",
            Error::InvalidExpression(_) => "Invalid opcode in DWARF expression",
            Error::InvalidPiece => {
                "DWARF expression has piece followed by non-piece expression at end"
            }
            Error::InvalidExpressionTerminator(_) => "Expected DW_OP_piece or DW_OP_bit_piece",
            Error::DivisionByZero => "Division or modulus by zero when evaluating expression",
            Error::TypeMismatch => "Type mismatch when evaluating expression",
            Error::IntegralTypeRequired => "Integral type expected when evaluating expression",
            Error::UnsupportedTypeOperation => {
                "An expression operation used types that are not supported"
            }
            Error::InvalidShiftExpression => {
                "The shift value in an expression must be a non-negative integer."
            }
            Error::UnknownCallFrameInstruction(_) => "An unknown DW_CFA_* instructiion",
            Error::InvalidAddressRange => {
                "The end of an address range must not be before the beginning."
            }
            Error::InvalidLocationAddressRange => {
                "The end offset of a location list entry must not be before the beginning."
            }
            Error::CfiInstructionInInvalidContext => {
                "Encountered a call frame instruction in a context in which it is not valid."
            }
            Error::PopWithEmptyStack => {
                "When evaluating call frame instructions, found a `DW_CFA_restore_state` stack pop \
                 instruction, but the stack was empty, and had nothing to pop."
            }
            Error::NoUnwindInfoForAddress => "Do not have unwind info for the given address.",
            Error::UnsupportedOffset => {
                "An offset value was larger than the maximum supported value."
            }
            Error::UnknownPointerEncoding => {
                "The given pointer encoding is either unknown or invalid."
            }
            Error::NoEntryAtGivenOffset => "Did not find an entry at the given offset.",
            Error::OffsetOutOfBounds => "The given offset is out of bounds.",
            Error::UnknownAugmentation => "Found an unknown CFI augmentation.",
            Error::UnsupportedPointerEncoding => {
                "We do not support the given pointer encoding yet."
            }
            Error::UnsupportedRegister(_) => "Registers larger than `u16` are not supported.",
            Error::TooManyRegisterRules => {
                "The CFI program defined more register rules than we have storage for."
            }
            Error::CfiStackFull => {
                "Attempted to push onto the CFI stack, but it was already at full capacity."
            }
            Error::VariableLengthSearchTable => {
                "The `.eh_frame_hdr` binary search table claims to be variable-length encoded, \
                 which makes binary search impossible."
            }
            Error::UnsupportedUnitType => "The `DW_UT_*` value for this unit is not supported yet",
            Error::UnsupportedAddressIndex => "Ranges involving AddressIndex are not supported yet",
            Error::UnsupportedSegmentSize => "Nonzero segment size not supported yet",
        }
    }
}

#[cfg(feature = "std")]
impl error::Error for Error {
    fn description(&self) -> &str {
        Error::description(self)
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    fn from(_: io::Error) -> Self {
        Error::Io
    }
}

/// The result of a parse.
pub type Result<T> = result::Result<T, Error>;

/// A convenience trait for loading DWARF sections from object files.  To be
/// used like:
///
/// ```
/// use gimli::{DebugInfo, EndianBuf, LittleEndian, Reader, Section};
///
/// fn load_section<R, S, F>(loader: F) -> S
///   where R: Reader, S: Section<R>, F: FnOnce(&'static str) -> R
/// {
///   let data = loader(S::section_name());
///   S::from(data)
/// }
///
/// let buf = [0x00, 0x01, 0x02, 0x03];
/// let reader = EndianBuf::new(&buf, LittleEndian);
///
/// let debug_info: DebugInfo<_> = load_section(|_: &'static str| reader);
/// ```
pub trait Section<R: Reader>: From<R> {
    /// Returns the ELF section name for this type.
    fn section_name() -> &'static str;
}

/// Parse a `DW_EH_PE_*` pointer encoding.
#[doc(hidden)]
#[inline]
pub(crate) fn parse_pointer_encoding<R: Reader>(input: &mut R) -> Result<constants::DwEhPe> {
    let eh_pe = input.read_u8()?;
    let eh_pe = constants::DwEhPe(eh_pe);

    if eh_pe.is_valid_encoding() {
        Ok(eh_pe)
    } else {
        Err(Error::UnknownPointerEncoding)
    }
}

/// A decoded pointer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Pointer {
    /// This value is the decoded pointer value.
    Direct(u64),

    /// This value is *not* the pointer value, but points to the address of
    /// where the real pointer value lives. In other words, deref this pointer
    /// to get the real pointer value.
    ///
    /// Chase this pointer at your own risk: do you trust the DWARF data it came
    /// from?
    Indirect(u64),
}

impl Default for Pointer {
    #[inline]
    fn default() -> Self {
        Pointer::Direct(0)
    }
}

impl Into<u64> for Pointer {
    #[inline]
    fn into(self) -> u64 {
        match self {
            Pointer::Direct(p) | Pointer::Indirect(p) => p,
        }
    }
}

impl Pointer {
    #[inline]
    fn new(encoding: constants::DwEhPe, address: u64) -> Pointer {
        if encoding.is_indirect() {
            Pointer::Indirect(address)
        } else {
            Pointer::Direct(address)
        }
    }
}

pub(crate) fn parse_encoded_pointer<'bases, R: Reader>(
    encoding: constants::DwEhPe,
    bases: &'bases SectionBaseAddresses,
    address_size: u8,
    section: &R,
    input: &mut R,
) -> Result<Pointer> {
    fn parse_data<R: Reader>(
        encoding: constants::DwEhPe,
        address_size: u8,
        input: &mut R,
    ) -> Result<u64> {
        // We should never be called with an invalid encoding: parse_encoded_pointer
        // checks validity for us.
        debug_assert!(encoding.is_valid_encoding());

        match encoding.format() {
            // Unsigned variants.
            constants::DW_EH_PE_absptr => input.read_address(address_size),
            constants::DW_EH_PE_uleb128 => input.read_uleb128(),
            constants::DW_EH_PE_udata2 => input.read_u16().map(u64::from),
            constants::DW_EH_PE_udata4 => input.read_u32().map(u64::from),
            constants::DW_EH_PE_udata8 => input.read_u64(),

            // Signed variants. Here we sign extend the values (happens by
            // default when casting a signed integer to a larger range integer
            // in Rust), return them as u64, and rely on wrapping addition to do
            // the right thing when adding these offsets to their bases.
            constants::DW_EH_PE_sleb128 => input.read_sleb128().map(|a| a as u64),
            constants::DW_EH_PE_sdata2 => input.read_i16().map(|a| a as u64),
            constants::DW_EH_PE_sdata4 => input.read_i32().map(|a| a as u64),
            constants::DW_EH_PE_sdata8 => input.read_i64().map(|a| a as u64),

            // That was all of the valid encoding formats.
            _ => unreachable!(),
        }
    }

    if !encoding.is_valid_encoding() {
        return Err(Error::UnknownPointerEncoding);
    }

    if encoding == constants::DW_EH_PE_omit {
        return Ok(Pointer::Direct(0));
    }

    match encoding.application() {
        constants::DW_EH_PE_absptr => {
            let addr = parse_data(encoding, address_size, input)?;
            Ok(Pointer::new(encoding, addr))
        }
        constants::DW_EH_PE_pcrel => {
            if let Some(section_base) = bases.section {
                let offset_from_section = input.offset_from(section);
                let offset = parse_data(encoding, address_size, input)?;
                let p = section_base
                    .wrapping_add(offset_from_section)
                    .wrapping_add(offset);
                Ok(Pointer::new(encoding, p))
            } else {
                Err(Error::PcRelativePointerButSectionBaseIsUndefined)
            }
        }
        constants::DW_EH_PE_textrel => {
            if let Some(text) = bases.text {
                let offset = parse_data(encoding, address_size, input)?;
                Ok(Pointer::new(encoding, text.wrapping_add(offset)))
            } else {
                Err(Error::TextRelativePointerButTextBaseIsUndefined)
            }
        }
        constants::DW_EH_PE_datarel => {
            if let Some(data) = bases.data {
                let offset = parse_data(encoding, address_size, input)?;
                Ok(Pointer::new(encoding, data.wrapping_add(offset)))
            } else {
                Err(Error::DataRelativePointerButDataBaseIsUndefined)
            }
        }
        constants::DW_EH_PE_funcrel => {
            let func = bases.func.borrow();
            if let Some(func) = *func {
                let offset = parse_data(encoding, address_size, input)?;
                Ok(Pointer::new(encoding, func.wrapping_add(offset)))
            } else {
                Err(Error::FuncRelativePointerInBadContext)
            }
        }
        constants::DW_EH_PE_aligned => Err(Error::UnsupportedPointerEncoding),
        _ => unreachable!(),
    }
}

impl Register {
    pub(crate) fn from_u64(x: u64) -> Result<Register> {
        let y = x as u16;
        if u64::from(y) == x {
            Ok(Register(y))
        } else {
            Err(Error::UnsupportedRegister(x))
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test_assembler;

    use self::test_assembler::{Endian, Section};
    use super::*;
    use common::Format;
    use constants;
    use endianity::LittleEndian;
    use std::cell::RefCell;
    use test_util::GimliSectionMethods;

    #[test]
    fn test_parse_initial_length_32_ok() {
        let section = Section::with_endian(Endian::Little).L32(0x7856_3412);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Ok((length, format)) => {
                assert_eq!(input.len(), 0);
                assert_eq!(format, Format::Dwarf32);
                assert_eq!(0x7856_3412, length);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    fn test_parse_initial_length_64_ok() {
        let section = Section::with_endian(Endian::Little)
            // Dwarf_64_INITIAL_UNIT_LENGTH
            .L32(0xffff_ffff)
            // Actual length
            .L64(0xffde_bc9a_7856_3412);
        let buf = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&buf, LittleEndian);

        #[cfg(target_pointer_width = "64")]
        match input.read_initial_length() {
            Ok((length, format)) => {
                assert_eq!(input.len(), 0);
                assert_eq!(format, Format::Dwarf64);
                assert_eq!(0xffde_bc9a_7856_3412, length);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }

        #[cfg(target_pointer_width = "32")]
        match input.read_initial_length() {
            Err(Error::UnsupportedOffset) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_initial_length_unknown_reserved_value() {
        let section = Section::with_endian(Endian::Little).L32(0xffff_fffe);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Err(Error::UnknownReservedLength) => assert!(true),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_initial_length_incomplete() {
        let buf = [0xff, 0xff, 0xff]; // Need at least 4 bytes.

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Err(Error::UnexpectedEof) => assert!(true),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_initial_length_64_incomplete() {
        let section = Section::with_endian(Endian::Little)
            // Dwarf_64_INITIAL_UNIT_LENGTH
            .L32(0xffff_ffff)
            // Actual length is not long enough.
            .L32(0x7856_3412);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Err(Error::UnexpectedEof) => assert!(true),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_offset_32() {
        let section = Section::with_endian(Endian::Little).L32(0x0123_4567);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf32) {
            Ok(val) => {
                assert_eq!(input.len(), 0);
                assert_eq!(val, 0x0123_4567);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_offset_64_small() {
        let section = Section::with_endian(Endian::Little).L64(0x0123_4567);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf64) {
            Ok(val) => {
                assert_eq!(input.len(), 0);
                assert_eq!(val, 0x0123_4567);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_offset_64_large() {
        let section = Section::with_endian(Endian::Little).L64(0x0123_4567_89ab_cdef);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf64) {
            Ok(val) => {
                assert_eq!(input.len(), 0);
                assert_eq!(val, 0x0123_4567_89ab_cdef);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_parse_offset_64_large() {
        let section = Section::with_endian(Endian::Little).L64(0x0123_4567_89ab_cdef);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf64) {
            Err(Error::UnsupportedOffset) => assert!(true),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_pointer_encoding_ok() {
        use endianity::NativeEndian;
        let expected =
            constants::DwEhPe(constants::DW_EH_PE_uleb128.0 | constants::DW_EH_PE_pcrel.0);
        let input = [expected.0, 1, 2, 3, 4];
        let input = &mut EndianSlice::new(&input, NativeEndian);
        assert_eq!(parse_pointer_encoding(input), Ok(expected));
        assert_eq!(*input, EndianSlice::new(&[1, 2, 3, 4], NativeEndian));
    }

    #[test]
    fn test_parse_pointer_encoding_bad_encoding() {
        use endianity::NativeEndian;
        let expected =
            constants::DwEhPe((constants::DW_EH_PE_sdata8.0 + 1) | constants::DW_EH_PE_pcrel.0);
        let input = [expected.0, 1, 2, 3, 4];
        let input = &mut EndianSlice::new(&input, NativeEndian);
        assert_eq!(
            Err(Error::UnknownPointerEncoding),
            parse_pointer_encoding(input)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_absptr() {
        let encoding = constants::DW_EH_PE_absptr;
        let bases = Default::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0xf00d_f00d)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0xf00d_f00d))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_pcrel() {
        let encoding = constants::DW_EH_PE_pcrel;

        let bases = BaseAddresses::default().set_eh_frame(0x100);

        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .append_repeated(0, 0x10)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input.range_from(0x10..);

        assert_eq!(
            parse_encoded_pointer(encoding, &bases.eh_frame, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x111))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_pcrel_undefined() {
        let encoding = constants::DW_EH_PE_pcrel;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Err(Error::PcRelativePointerButSectionBaseIsUndefined)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_textrel() {
        let encoding = constants::DW_EH_PE_textrel;

        let bases = BaseAddresses::default().set_text(0x10);

        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases.eh_frame, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x11))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_textrel_undefined() {
        let encoding = constants::DW_EH_PE_textrel;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Err(Error::TextRelativePointerButTextBaseIsUndefined)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_datarel() {
        let encoding = constants::DW_EH_PE_datarel;

        let bases = BaseAddresses::default().set_got(0x10);

        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases.eh_frame, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x11))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_datarel_undefined() {
        let encoding = constants::DW_EH_PE_datarel;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Err(Error::DataRelativePointerButDataBaseIsUndefined)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_funcrel() {
        let encoding = constants::DW_EH_PE_funcrel;

        let mut bases = SectionBaseAddresses::default();
        bases.func = RefCell::new(Some(0x10));

        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x11))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_funcrel_undefined() {
        let encoding = constants::DW_EH_PE_funcrel;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Err(Error::FuncRelativePointerInBadContext)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_uleb128() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_uleb128.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .uleb(0x12_3456)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x12_3456))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_udata2() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_udata2.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L16(0x1234)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x1234))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_udata4() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_udata4.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1234_5678)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x1234_5678))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_udata8() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_udata8.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L64(0x1234_5678_1234_5678)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x1234_5678_1234_5678))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sleb128() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_textrel.0 | constants::DW_EH_PE_sleb128.0);
        let bases = BaseAddresses::default().set_text(0x1111_1111);
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .sleb(-0x1111)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases.eh_frame, address_size, &input, &mut rest),
            Ok(Pointer::Direct(0x1111_0000))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sdata2() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_sdata2.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];
        let expected = 0x111 as i16;

        let input = Section::with_endian(Endian::Little)
            .L16(expected as u16)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(expected as u64))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sdata4() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_sdata4.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];
        let expected = 0x111_1111 as i32;

        let input = Section::with_endian(Endian::Little)
            .L32(expected as u32)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(expected as u64))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sdata8() {
        let encoding =
            constants::DwEhPe(constants::DW_EH_PE_absptr.0 | constants::DW_EH_PE_sdata8.0);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;
        let expected_rest = [1, 2, 3, 4];
        let expected = -0x11_1111_1222_2222 as i64;

        let input = Section::with_endian(Endian::Little)
            .L64(expected as u64)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Direct(expected as u64))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_omit() {
        let encoding = constants::DW_EH_PE_omit;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::default())
        );
        assert_eq!(rest, input);
    }

    #[test]
    fn test_parse_encoded_pointer_bad_encoding() {
        let encoding = constants::DwEhPe(constants::DW_EH_PE_sdata8.0 + 1);
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Err(Error::UnknownPointerEncoding)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_aligned() {
        // FIXME: support this encoding!

        let encoding = constants::DW_EH_PE_aligned;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Err(Error::UnsupportedPointerEncoding)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_indirect() {
        let expected_rest = [1, 2, 3, 4];

        let encoding = constants::DW_EH_PE_indirect;
        let bases = SectionBaseAddresses::default();
        let address_size = 4;

        let input = Section::with_endian(Endian::Little)
            .L32(0x1234_5678)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        assert_eq!(
            parse_encoded_pointer(encoding, &bases, address_size, &input, &mut rest),
            Ok(Pointer::Indirect(0x1234_5678))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }
}
