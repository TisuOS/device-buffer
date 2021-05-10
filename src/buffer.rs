use core::{cmp::min, usize};

use tisu_driver::BlockDriver;
use tisu_sync::{Bool, ReadWriteMutex};

/// ## 磁盘缓冲
/// 当前缓冲池使用的是普通自旋锁，没有禁止中断的功能，存在死锁风险
/// 应当在后续中将读写锁替换成 tisu-sync, tag=v2.0 的 lock_no_int 系列
pub struct Buffer {
    pub use_cnt : usize,
    pub offset : usize,
    pub size : usize,
    is_write : Bool,
    data : &'static mut [u8],
    mutex : ReadWriteMutex,
}

impl Buffer {
    pub fn new(
        offset : usize,
        driver : &mut dyn BlockDriver,
        size : usize,
        data : &'static mut [u8],
    )->Self {
        driver.sync_read(offset, size, data).unwrap();
        Self {
            mutex: ReadWriteMutex::new(true),
            use_cnt: 0,
            offset,
            size,
            data,
            is_write: Bool::new(),
        }
    }

    pub fn write(&mut self, data:&[u8], st:usize)->CopyResult {
        self.use_cnt = self.use_cnt.wrapping_add(1);
        let offset = st % self.size;
        let end = offset + data.len();
        let ed = min(end, self.size);
        let buffer_data = &mut self.data[offset..ed];
        let data = &data[..(min(end, ed) - offset)];
        self.mutex.read();
        buffer_data.copy_from_slice(data);
        self.mutex.unlock();
        if end > ed { CopyResult::TooLong }
        else { CopyResult::Finish }
    }

    pub fn read(&mut self, data:&mut [u8], st:usize)->CopyResult {
        self.use_cnt = self.use_cnt.wrapping_add(1);
        let offset = st % self.size;
        let end = offset + data.len();
        let ed = min(end, self.size);
        let buffer_data = &mut self.data[offset..ed];
        let data = &mut data[..(min(end, ed) - offset)];
        self.mutex.read();
        data.copy_from_slice(buffer_data);
        self.mutex.unlock();
        if end > ed { CopyResult::TooLong }
        else { CopyResult::Finish }
    }

    pub fn refresh(&mut self, offset : usize, driver : &mut dyn BlockDriver) {
        let offset = offset / self.size * self.size;
        self.mutex.write();
        self.swap(driver);
        self.offset = offset;
        driver.sync_read(self.offset, self.size, self.data).unwrap();
        self.mutex.unlock();
    }

    fn swap(&mut self, driver : &mut dyn BlockDriver) {
        assert!(0 == 0);
        if self.is_write.get_val() {
            driver.sync_write(self.offset, self.size, self.data).unwrap();
        }
    }
}

pub enum CopyResult {
    Finish,
    TooLong,
}