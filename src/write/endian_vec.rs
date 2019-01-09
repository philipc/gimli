use vec::Vec;

use endianity::Endianity;
use write::{Error, Result, Writer, WriterOffset};

/// A `Vec<u8>` with endianity metadata.
///
/// This implements the `Writer` trait, which is used for all writing of DWARF sections.
#[derive(Debug, Clone)]
pub struct EndianVec<Endian>
where
    Endian: Endianity,
{
    vec: Vec<u8>,
    endian: Endian,
}

impl<Endian> EndianVec<Endian>
where
    Endian: Endianity,
{
    /// Construct an empty `EndianVec` with the given endianity.
    pub fn new(endian: Endian) -> EndianVec<Endian> {
        EndianVec {
            vec: Vec::new(),
            endian,
        }
    }

    /// Return a reference to the raw slice.
    pub fn slice(&self) -> &[u8] {
        &self.vec
    }

    /// Convert into a `Vec<u8>`.
    pub fn into_vec(self) -> Vec<u8> {
        self.vec
    }
}

impl<Endian> Writer for EndianVec<Endian>
where
    Endian: Endianity,
{
    type Endian = Endian;

    #[inline]
    fn endian(&self) -> Self::Endian {
        self.endian
    }

    #[inline]
    fn len(&self) -> WriterOffset {
        self.vec.len() as WriterOffset
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.vec.extend(bytes);
        Ok(())
    }

    fn write_at(&mut self, offset: WriterOffset, bytes: &[u8]) -> Result<()> {
        let offset_usize = offset as usize;
        if offset_usize as WriterOffset != offset {
            return Err(Error::OffsetOutOfBounds);
        }
        if offset_usize > self.vec.len() {
            return Err(Error::OffsetOutOfBounds);
        }
        let to = &mut self.vec[offset_usize..];
        if bytes.len() > to.len() {
            return Err(Error::LengthOutOfBounds);
        }
        let to = &mut to[..bytes.len()];
        to.copy_from_slice(bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LittleEndian;

    #[test]
    fn test_endian_vec() {
        let mut w = EndianVec::new(LittleEndian);
        assert_eq!(w.endian(), LittleEndian);
        assert_eq!(w.len(), 0);

        w.write(&[1, 2]).unwrap();
        assert_eq!(w.slice(), &[1, 2]);
        assert_eq!(w.len(), 2);

        w.write(&[3, 4, 5]).unwrap();
        assert_eq!(w.slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(w.len(), 5);

        w.write_at(0, &[6, 7]).unwrap();
        assert_eq!(w.slice(), &[6, 7, 3, 4, 5]);
        assert_eq!(w.len(), 5);

        w.write_at(3, &[8, 9]).unwrap();
        assert_eq!(w.slice(), &[6, 7, 3, 8, 9]);
        assert_eq!(w.len(), 5);

        assert_eq!(w.write_at(4, &[6, 7]), Err(Error::LengthOutOfBounds));
        assert_eq!(w.write_at(5, &[6, 7]), Err(Error::LengthOutOfBounds));
        assert_eq!(w.write_at(6, &[6, 7]), Err(Error::OffsetOutOfBounds));

        assert_eq!(w.into_vec(), vec![6, 7, 3, 8, 9]);
    }
}
