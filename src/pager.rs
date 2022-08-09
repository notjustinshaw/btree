use crate::error::Error;
use crate::node_type::Offset;
use crate::page::Page;
use crate::page_layout::PAGE_SIZE;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// A utility for reading and writing pages to a file.
pub struct Pager {
    file: File,
    curser: usize,
}

impl Pager {
    /// Creates a new pager for the given file with offset 0.
    pub fn new(path: &Path) -> Result<Pager, Error> {
        let fd = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        Ok(Pager {
            file: fd,
            curser: 0,
        })
    }

    /// Reads a single page from the file starting at the given offset.
    pub fn get_page(&mut self, offset: &Offset) -> Result<Page, Error> {
        let mut page: [u8; PAGE_SIZE] = [0x00; PAGE_SIZE];
        self.file.seek(SeekFrom::Start(offset.0 as u64))?;
        self.file.read_exact(&mut page)?;
        Ok(Page::new(page))
    }

    /// Writes the given page to the file at the current cursor position and
    /// returns the offset of the new page (ie. the old cursor position).
    ///
    /// The current cursor position is an offset from the start of the page
    /// that is incremented on each call to this function (initially 0).
    pub fn write_page(&mut self, page: Page) -> Result<Offset, Error> {
        self.file.seek(SeekFrom::Start(self.curser as u64))?;
        self.file.write_all(&page.get_data())?;
        let res = Offset(self.curser);
        self.curser += PAGE_SIZE;
        Ok(res)
    }

    /// Writes the given page to the file at the given offset.
    pub fn write_page_at_offset(&mut self, page: Page, offset: &Offset) -> Result<(), Error> {
        self.file.seek(SeekFrom::Start(offset.0 as u64))?;
        self.file.write_all(&page.get_data())?;
        Ok(())
    }
}
