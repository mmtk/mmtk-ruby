pub struct ChunkedVecCollector<T> {
    vecs: Vec<Vec<T>>,
    current_vec: Vec<T>,
    chunk_size: usize,
}

impl<T> ChunkedVecCollector<T> {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            vecs: vec![],
            current_vec: Vec::with_capacity(chunk_size),
            chunk_size,
        }
    }

    pub fn add(&mut self, item: T) {
        self.current_vec.push(item);
        if self.current_vec.len() == self.chunk_size {
            self.flush();
        }
    }

    fn flush(&mut self) {
        let new_vec = Vec::with_capacity(self.chunk_size);
        let old_vec = std::mem::replace(&mut self.current_vec, new_vec);
        self.vecs.push(old_vec);
    }

    pub fn into_vecs(mut self) -> Vec<Vec<T>> {
        if !self.current_vec.is_empty() {
            self.flush();
        }
        self.vecs
    }
}

impl<A> Extend<A> for ChunkedVecCollector<A> {
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        for item in iter {
            self.add(item);
        }
    }
}
