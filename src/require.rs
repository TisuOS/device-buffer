pub trait CacheBuffer {
    fn read(&mut self, device_id : usize, data:&mut [u8], st:usize);
    fn write(&mut self, device_id : usize, data:& [u8], st:usize);
}