use core::{ptr::slice_from_raw_parts, usize};

use crate::{CacheBuffer, buffer::Buffer};
use alloc::{collections::BTreeMap, vec::Vec};
use tisu_driver::BlockDriver;
use tisu_memory::MemoryOp;

#[allow(dead_code)]
pub struct Cache {
    cache_size : usize,
    buffer_size : usize,
    driver : BTreeMap<usize, &'static mut dyn BlockDriver>,
    buffer : BTreeMap<usize, Vec<Buffer>>,
}

impl Cache {
    pub fn new(cache_size : usize, buffer_size : usize)->Self {
        Self {
            cache_size,
            buffer_size,
            driver: BTreeMap::new(),
            buffer: BTreeMap::new(),
        }
    }

    pub fn add_buffer(
        &mut self, device_id : usize,
        driver: &'static mut dyn BlockDriver,
        memory: &'static mut impl MemoryOp
    ) {
        self.buffer.insert(device_id, Vec::new());
        let buffer = self.buffer.get_mut(&device_id).unwrap();
        for i in 0..self.cache_size {
            let data = memory.alloc_memory(self.buffer_size, true).unwrap();
            let data = slice_from_raw_parts(data, self.buffer_size);
            let data = unsafe{&mut *(data as *mut [u8])};
            buffer.push(Buffer::new(
                i * self.buffer_size, driver, self.buffer_size, data));
        }
        self.driver.insert(device_id, driver);
    }

    fn refresh(&mut self, device_id : usize, st:usize) {
        let buffer = self.buffer.get_mut(&device_id).unwrap();
        buffer.sort_by(|a, b| {
            a.use_cnt.cmp(&b.use_cnt)
        });
        for (i, buf) in buffer.iter_mut().enumerate() {
            buf.use_cnt = i;
        }
        let buf = buffer.first_mut().unwrap();
        let t = self.driver.get_mut(&device_id).unwrap();
        buf.refresh(st, *t);
    }
}

impl CacheBuffer for Cache {
    fn read(&mut self, device_id : usize, data:&mut [u8], st:usize) {
        let mut buffer = self.buffer.get_mut(&device_id).unwrap();
        let mut buf = buffer.iter_mut().find(|buf|{
            buf.offset == st / buf.size * buf.size
        });
        if buf.is_none() {
            self.refresh(device_id, st);
            buffer = self.buffer.get_mut(&device_id).unwrap();
            buf = buffer.iter_mut().find(|buf|{
                buf.offset == st / buf.size * buf.size
            });
        }
        let buf = buf.unwrap();
        match buf.read(data, st) {
            crate::buffer::CopyResult::Finish => {}
            crate::buffer::CopyResult::TooLong => {
                let data = &mut data[(buf.offset + buf.size - st)..];
                let st = buf.offset + buf.size;
                self.read(device_id, data, st)
            }
        }
    }

    fn write(&mut self, device_id : usize, data:& [u8], st:usize) {
        let mut buffer = self.buffer.get_mut(&device_id).unwrap();
        let mut buf = buffer.iter_mut().find(|buf|{
            buf.offset == st / buf.size * buf.size
        });
        if buf.is_none() {
            self.refresh(device_id, st);
            buffer = self.buffer.get_mut(&device_id).unwrap();
            buf = buffer.iter_mut().find(|buf|{
                buf.offset == st / buf.size * buf.size
            });
        }
        let buf = buf.unwrap();
        match buf.write(data, st) {
            crate::buffer::CopyResult::Finish => {}
            crate::buffer::CopyResult::TooLong => {
                let data = &data[(buf.offset + buf.size - st)..];
                let st = buf.offset + buf.size;
                self.write(device_id, data, st)
            }
        }
    }
}
