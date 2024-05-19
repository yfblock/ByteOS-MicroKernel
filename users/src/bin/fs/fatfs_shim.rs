use users::syscall::block_read;

pub struct DiskCursor {
    pub sector: u64,
    pub offset: usize,
    pub blk_tid: usize,
}

unsafe impl Sync for DiskCursor {}
unsafe impl Send for DiskCursor {}

impl DiskCursor {
    fn get_position(&self) -> usize {
        (self.sector * 0x200) as usize + self.offset
    }

    fn set_position(&mut self, position: usize) {
        self.sector = (position / 0x200) as u64;
        self.offset = position % 0x200;
    }

    fn move_cursor(&mut self, amount: usize) {
        self.set_position(self.get_position() + amount)
    }
}

impl fatfs::IoBase for DiskCursor {
    type Error = ();
}

impl fatfs::Read for DiskCursor {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // 由于读取扇区内容还需要考虑跨 cluster，因此 read 函数只读取一个扇区
        // 防止读取较多数据时超出限制
        // 读取所有的数据的功能交给 read_exact 来实现

        // 如果 start 不是 0 或者 len 不是 512
        // let device = get_blk_device(self.device_id).unwrap();
        let read_size = if self.offset != 0 || buf.len() < 512 {
            let mut data = vec![0u8; 512];
            // device.read_blocks(self.sector as usize, &mut data);
            block_read(self.blk_tid, self.sector as _, &mut data);

            let start = self.offset;
            let end = (self.offset + buf.len()).min(512);

            buf[..end - start].copy_from_slice(&data[start..end]);
            end - start
        } else {
            // floor the buf len
            let rlen = (buf.len() / 512) * 512;
            assert!(rlen % 0x200 == 0);
            // 如果不用同一个数组 会导致读取数据的时候出现问题
            block_read(self.blk_tid, self.sector as _, buf);
            rlen
        };
        self.move_cursor(read_size);
        Ok(read_size)
    }
}

// TODO: implement write function
impl fatfs::Write for DiskCursor {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        // 由于写入扇区还需要考虑申请 cluster，因此 write 函数只写入一个扇区
        // 防止写入较多数据时超出限制
        // 写入所有的数据的功能交给 write_all 来实现

        // 获取硬盘设备写入器（驱动？）
        // 如果 start 不是 0 或者 len 不是 512
        // let device = get_blk_device(self.device_id).unwrap();
        // let write_size = if self.offset != 0 || buf.len() < 512 {
        //     let mut data = vec![0u8; 512];
        //     device.read_blocks(self.sector as usize, &mut data);

        //     let start = self.offset;
        //     let end = (self.offset + buf.len()).min(512);

        //     data[start..end].clone_from_slice(&buf[..end - start]);
        //     device.write_blocks(self.sector as usize, &mut data);

        //     end - start
        // } else {
        //     // should copy data from buffer
        //     let mut data = vec![0u8; 512];
        //     data.copy_from_slice(&buf[..512]);
        //     device.write_blocks(self.sector as usize, &data);
        //     512
        // };

        // self.move_cursor(write_size);
        // Ok(write_size)
        unimplemented!("write data to block device")
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl fatfs::Seek for DiskCursor {
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            fatfs::SeekFrom::Start(i) => {
                self.set_position(i as usize);
                Ok(i)
            }
            fatfs::SeekFrom::End(_) => unreachable!(),
            fatfs::SeekFrom::Current(i) => {
                let new_pos = (self.get_position() as i64) + i;
                self.set_position(new_pos as usize);
                Ok(new_pos as u64)
            }
        }
    }
}
