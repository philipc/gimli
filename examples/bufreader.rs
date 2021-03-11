//! A simple example of parsing `.debug_info`.

use gimli::Reader;
use object::{Object, ObjectSection};
use std::cell::RefCell;
use std::convert::TryInto;
use std::io::{Read, Seek};
use std::{borrow, cmp, env, fs, io};

fn main() {
    for path in env::args().skip(1) {
        let file = fs::File::open(&path).unwrap();
        dump_file(file).unwrap();
    }
}

fn dump_file(file: fs::File) -> Result<(), gimli::Error> {
    // Parse the file and locate the DWARF sections.
    let reader = object::ReadCache::new(file);
    let object = object::File::parse(&reader).unwrap();
    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    // Locate the file range for a section's data.
    let locate_section =
        |id: gimli::SectionId| -> Result<object::CompressedFileRange, gimli::Error> {
            match object.section_by_name(id.name()) {
                Some(ref section) => Ok(section.compressed_file_range().unwrap()),
                None => Ok(object::CompressedFileRange::none(None)),
            }
        };
    // Locate a supplementary section. We don't have a supplementary object file,
    // so always return an empty range.
    let locate_section_sup = |_| Ok(object::CompressedFileRange::none(None));

    // Locate all of the sections.
    let dwarf_file_range = gimli::Dwarf::load(&locate_section, &locate_section_sup)?;

    // Done parsing the object.
    let file = reader.into_inner();

    // Create buffers for each DWARF section.
    let dwarf_buffer = dwarf_file_range.borrow(|range| {
        // TODO: handle compressed sections
        assert!(range.format == object::CompressionFormat::None);
        RefCell::new(SectionBuffer::new(
            &file,
            range.offset,
            range.uncompressed_size,
        ))
    });
    // And now create cloneable references to those buffers.
    let dwarf = dwarf_buffer.borrow(|buffer| {
        let range_start = buffer.borrow().file_offset;
        let range_size = buffer.borrow().file_size;
        SectionReader {
            buffer,
            endian,
            range_start,
            range_size,
        }
    });

    // Iterate over the compilation units.
    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        println!(
            "Unit at <.debug_info+0x{:x}>",
            header.offset().as_debug_info_offset().unwrap().0
        );
        let unit = dwarf.unit(header)?;

        // Iterate over the Debugging Information Entries (DIEs) in the unit.
        let mut depth = 0;
        let mut entries = unit.entries();
        while let Some((delta_depth, entry)) = entries.next_dfs()? {
            depth += delta_depth;
            println!("<{}><{:x}> {}", depth, entry.offset().0, entry.tag());

            // Iterate over the attributes in the DIE.
            let mut attrs = entry.attrs();
            while let Some(attr) = attrs.next()? {
                print!("   {}: ", attr.name());
                match attr.value() {
                    gimli::AttributeValue::Block(r) => {
                        println!("Block({:?})", r.to_slice()?);
                    }
                    gimli::AttributeValue::Exprloc(e) => {
                        println!("Exprloc({:?})", e.0.to_slice()?);
                    }
                    gimli::AttributeValue::String(r) => {
                        println!("String({})", r.to_string_lossy()?);
                    }
                    val => println!("{:?}", val),
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct SectionBuffer<'a> {
    file: &'a fs::File,
    file_offset: u64,
    file_size: u64,
    buf: Box<[u8]>,
    buf_offset: u64,
    buf_size: usize,
}

impl<'a> SectionBuffer<'a> {
    fn new(file: &'a fs::File, file_offset: u64, file_size: u64) -> Self {
        SectionBuffer {
            file,
            file_offset,
            file_size,
            buf: vec![0; 8192].into_boxed_slice(),
            buf_offset: file_offset,
            buf_size: 0,
        }
    }

    fn read_bytes_at(&mut self, file_offset: u64, buf: &mut [u8]) -> Result<(), gimli::Error> {
        // Check if completely in the buffer.
        // TODO: use partial reads if available.
        if let Some(Ok(buf_start)) = file_offset
            .checked_sub(self.buf_offset)
            .map(|x| x.try_into())
        {
            if let Some(buf_remaining) = self.buf_size.checked_sub(buf_start) {
                if buf_remaining >= buf.len() {
                    // Fast path: already in the buffer.
                    buf.clone_from_slice(&self.buf[buf_start..][..buf.len()]);
                    return Ok(());
                }
            }
        }

        // Check the requested range is valid.
        let section_offset = file_offset
            .checked_sub(self.file_offset)
            .ok_or(gimli::Error::Io)?;
        let remaining = self
            .file_size
            .checked_sub(section_offset)
            .ok_or(gimli::Error::Io)?;
        if buf.len() as u64 > remaining {
            return Err(gimli::Error::Io);
        }

        // Read directly into large buffers.
        if buf.len() >= self.buf.len() {
            self.file
                .seek(io::SeekFrom::Start(file_offset))
                .map_err(|_| gimli::Error::Io)?;
            self.file.read_exact(buf).map_err(|_| gimli::Error::Io)?;
            return Ok(());
        }

        // Read as much as we can into the buffer.
        let buf_size = cmp::min(self.buf.len() as u64, remaining) as usize;
        //println!("read {}[{}]", file_offset, buf_size);
        // TODO: use rounded offset?
        // TODO: avoid unneeded seeks
        self.file
            .seek(io::SeekFrom::Start(file_offset))
            .map_err(|_| gimli::Error::Io)?;
        self.file
            .read_exact(&mut self.buf[..buf_size])
            .map_err(|_| gimli::Error::Io)?;
        self.buf_offset = file_offset;
        self.buf_size = buf_size;

        // Return only the part that was requested.
        buf.clone_from_slice(&self.buf[..buf.len()]);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct SectionReader<'a> {
    buffer: &'a RefCell<SectionBuffer<'a>>,
    endian: gimli::RunTimeEndian,
    range_start: u64,
    range_size: u64,
}

impl<'a> gimli::Reader for SectionReader<'a> {
    type Endian = gimli::RunTimeEndian;
    type Offset = u64;

    #[inline]
    fn endian(&self) -> gimli::RunTimeEndian {
        self.endian
    }

    #[inline]
    fn len(&self) -> u64 {
        self.range_size
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.range_size == 0
    }

    #[inline]
    fn empty(&mut self) {
        self.range_size = 0;
    }

    #[inline]
    fn truncate(&mut self, len: u64) -> gimli::Result<()> {
        if self.range_size < len {
            Err(gimli::Error::UnexpectedEof(self.offset_id()))
        } else {
            self.range_size = len;
            Ok(())
        }
    }

    #[inline]
    fn offset_from(&self, base: &Self) -> u64 {
        self.range_start - base.range_start
    }

    #[inline]
    fn offset_id(&self) -> gimli::ReaderOffsetId {
        gimli::ReaderOffsetId(self.range_start)
    }

    #[inline]
    fn lookup_offset_id(&self, id: gimli::ReaderOffsetId) -> Option<Self::Offset> {
        let offset = id.0.checked_sub(self.range_start)?;
        if offset < self.range_size {
            Some(offset)
        } else {
            None
        }
    }

    #[inline]
    fn find(&self, byte: u8) -> gimli::Result<u64> {
        // Read 4096-byte slices until the value is found.
        // TODO: Maybe make sure that chunks are aligned with 4096 chunks in the
        // original space?
        // TODO: peek at the buffered reader
        let mut buf = [0; 4096];
        let start = self.range_start;
        let end = self.range_start + self.range_size;
        let mut chunk_start = start;
        while chunk_start < end {
            let chunk_size = cmp::min(4096, end - chunk_start);
            let read_chunk = &mut buf[..chunk_size as usize];
            self.buffer
                .borrow_mut()
                .read_bytes_at(chunk_start, read_chunk)?;
            if let Some(pos) = read_chunk.iter().position(|b| *b == byte) {
                return Ok((chunk_start - start) + pos as u64);
            }
            chunk_start += chunk_size;
        }
        Err(gimli::Error::UnexpectedEof(self.offset_id()))
    }

    #[inline]
    fn skip(&mut self, len: u64) -> gimli::Result<()> {
        if self.range_size < len {
            Err(gimli::Error::UnexpectedEof(self.offset_id()))
        } else {
            self.range_start += len;
            self.range_size -= len;
            Ok(())
        }
    }

    #[inline]
    fn split(&mut self, len: u64) -> gimli::Result<Self> {
        if self.range_size < len {
            return Err(gimli::Error::UnexpectedEof(self.offset_id()));
        }
        let mut copy = *self;
        self.range_start += len;
        self.range_size -= len;
        copy.range_size = len;
        Ok(copy)
    }

    #[inline]
    fn to_slice(&self) -> gimli::Result<borrow::Cow<[u8]>> {
        // TODO: peek at the buffered reader
        let mut slice = vec![0; self.range_size as usize];
        self.buffer
            .borrow_mut()
            .read_bytes_at(self.range_start, &mut slice)?;
        Ok(slice.into())
    }

    #[inline]
    fn to_string(&self) -> gimli::Result<borrow::Cow<str>> {
        // TODO: peek at the buffered reader
        let mut slice = vec![0; self.range_size as usize];
        self.buffer
            .borrow_mut()
            .read_bytes_at(self.range_start, &mut slice)?;
        match String::from_utf8(slice) {
            Ok(s) => Ok(s.into()),
            _ => Err(gimli::Error::BadUtf8),
        }
    }

    #[inline]
    fn to_string_lossy(&self) -> gimli::Result<borrow::Cow<str>> {
        // TODO: peek at the buffered reader
        let mut slice = vec![0; self.range_size as usize];
        self.buffer
            .borrow_mut()
            .read_bytes_at(self.range_start, &mut slice)?;
        Ok(String::from_utf8_lossy(&slice).into_owned().into())
    }

    #[inline]
    fn read_slice(&mut self, buf: &mut [u8]) -> gimli::Result<()> {
        let size = buf.len() as u64;
        if self.range_size < size {
            return Err(gimli::Error::UnexpectedEof(self.offset_id()));
        }
        self.buffer
            .borrow_mut()
            .read_bytes_at(self.range_start, buf)?;
        self.range_start += size;
        self.range_size -= size;
        Ok(())
    }
}
