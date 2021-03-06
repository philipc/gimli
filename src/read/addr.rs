use common::{DebugAddrBase, DebugAddrIndex};
use read::{Reader, ReaderOffset, Result, Section};

/// The raw contents of the `.debug_addr` section.
#[derive(Debug, Default, Clone, Copy)]
pub struct DebugAddr<R: Reader> {
    section: R,
}

impl<R: Reader> DebugAddr<R> {
    // TODO: add an iterator over the sets of addresses in the section.
    // This is not needed for common usage of the section though.

    /// Returns the address at the given `base` and `index`.
    ///
    /// A set of addresses in the `.debug_addr` section consists of a header
    /// followed by a series of addresses.
    ///
    /// The `base` must be the `DW_AT_addr_base` value from the compilation unit DIE.
    /// This is an offset that points to the first address following the header.
    ///
    /// The `index` is the value of a `DW_FORM_addrx` attribute.
    ///
    /// The `address_size` must be the size of the address for the compilation unit.
    /// This value must also match the header. However, note that we do not parse the
    /// header to validate this, since locating the header is unreliable, and the GNU
    /// extensions do not emit it.
    pub fn get_address(
        &self,
        address_size: u8,
        base: DebugAddrBase<R::Offset>,
        index: DebugAddrIndex<R::Offset>,
    ) -> Result<u64> {
        let input = &mut self.section.clone();
        input.skip(base.0)?;
        input.skip(R::Offset::from_u64(
            index.0.into_u64() * u64::from(address_size),
        )?)?;
        input.read_address(address_size)
    }
}

impl<R: Reader> Section<R> for DebugAddr<R> {
    fn section_name() -> &'static str {
        ".debug_addr"
    }
}

impl<R: Reader> From<R> for DebugAddr<R> {
    fn from(section: R) -> Self {
        DebugAddr { section }
    }
}

#[cfg(test)]
mod tests {
    extern crate test_assembler;

    use self::test_assembler::{Endian, Label, LabelMaker, Section};
    use super::*;
    use read::EndianSlice;
    use test_util::GimliSectionMethods;
    use {Format, LittleEndian};

    #[test]
    fn test_get_address() {
        for format in vec![Format::Dwarf32, Format::Dwarf64] {
            for address_size in vec![4, 8] {
                let zero = Label::new();
                let length = Label::new();
                let start = Label::new();
                let first = Label::new();
                let end = Label::new();
                let mut section = Section::with_endian(Endian::Little)
                    .mark(&zero)
                    .initial_length(format, &length, &start)
                    .D16(5)
                    .D8(address_size)
                    .D8(0)
                    .mark(&first);
                for i in 0..20 {
                    section = section.word(address_size, 1000 + i);
                }
                section = section.mark(&end);
                length.set_const((&end - &start) as u64);

                let section = section.get_contents().unwrap();
                let debug_addr = DebugAddr::from(EndianSlice::new(&section, LittleEndian));
                let base = DebugAddrBase((&first - &zero) as usize);

                assert_eq!(
                    debug_addr.get_address(address_size, base, DebugAddrIndex(0)),
                    Ok(1000)
                );
                assert_eq!(
                    debug_addr.get_address(address_size, base, DebugAddrIndex(19)),
                    Ok(1019)
                );
            }
        }
    }
}
