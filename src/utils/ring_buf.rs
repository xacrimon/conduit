pub struct RingBuf {
    buf: Box<[u8]>,
    read_pos: usize,
    write_pos: usize,
}

impl RingBuf {
    pub fn new(size: usize) -> Self {
        Self {
            buf: vec![0; size].into_boxed_slice(),
            read_pos: 0,
            write_pos: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.read_pos == self.write_pos
    }

    pub fn size(&self) -> usize {
        self.buf.len()
    }

    pub fn writable_slice(&mut self) -> &mut [u8] {
        if self.write_pos >= self.read_pos {
            let end = if self.read_pos == 0 {
                self.size() - 1
            } else {
                self.size()
            };
            &mut self.buf[self.write_pos..end]
        } else {
            &mut self.buf[self.write_pos..self.read_pos - 1]
        }
    }

    pub fn advance_write(&mut self, n: usize) {
        self.write_pos = (self.write_pos + n) % self.size();
    }

    pub fn readable_slice(&self) -> &[u8] {
        if self.write_pos >= self.read_pos {
            &self.buf[self.read_pos..self.write_pos]
        } else {
            &self.buf[self.read_pos..]
        }
    }

    pub fn advance_read(&mut self, n: usize) {
        self.read_pos = (self.read_pos + n) % self.size();
    }
}
