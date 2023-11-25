use std::ops::{Deref, RangeBounds};

/// a buffer which can have index references.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexRefBuf {
    buf: Vec<u8>,
    references: Vec<usize>,
}
impl IndexRefBuf {
    /// creates a new empty buffer.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            references: Vec::new(),
        }
    }
    /// creates a new buffer with the given content.
    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self {
            buf: vec,
            references: Vec::new(),
        }
    }
    /// creates an index reference to the given index in the buffer.
    pub fn create_index_ref(&mut self, index: usize) -> IndexRef {
        let ref_index = self.references.len();
        self.references.push(index);
        IndexRef { ref_index }
    }
    /// reads the index of the given index ref.
    pub fn read_index_ref(&self, index_ref: IndexRef) -> usize {
        self.references[index_ref.ref_index]
    }
    /// push the given element to the buffer.
    pub fn push(&mut self, value: u8) {
        self.buf.push(value);
    }
    /// extend the buffer using the content of the given slice.
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        self.buf.extend_from_slice(other);
    }
    /// appends the given vector to the buffer.
    pub fn append(&mut self, other: &mut Vec<u8>) {
        self.buf.append(other)
    }
    /// inserts an element into the buffer at the given index.
    pub fn insert(&mut self, index: usize, element: u8) {
        self.buf.insert(index, element);
        for reference in &mut self.references {
            if *reference >= index {
                *reference += 1;
            }
        }
    }
    /// inserts a slice into the buffer at the given index.
    pub fn insert_slice(&mut self, index: usize, elements: &[u8]) {
        self.buf.splice(index..index, elements.iter().copied());
        for reference in &mut self.references {
            if *reference >= index {
                *reference += elements.len();
            }
        }
    }
    /// replaces the given range with the given content.
    pub fn splice<R, I, T>(
        &mut self,
        range: R,
        replace_with: I,
    ) -> std::vec::Splice<'_, I::IntoIter>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = u8, IntoIter = T>,
        T: Iterator<Item = u8> + ExactSizeIterator,
    {
        let range_start_index = match range.start_bound() {
            std::ops::Bound::Included(x) => *x,
            std::ops::Bound::Excluded(x) => *x + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let range_end_index = match range.end_bound() {
            std::ops::Bound::Included(x) => *x + 1,
            std::ops::Bound::Excluded(x) => *x,
            std::ops::Bound::Unbounded => self.buf.len(),
        };
        let replace_with_iter = replace_with.into_iter();
        let replace_with_len = replace_with_iter.len();
        let result = self.buf.splice(range, replace_with_iter);

        let range_len = range_end_index - range_start_index;
        let increase_in_size = replace_with_len
            .checked_sub(range_len)
            .expect("index referencable buffers may only grow, shrinking is not allowed");
        if increase_in_size > 0 {
            for reference in &mut self.references {
                if *reference >= range_end_index {
                    *reference += increase_in_size;
                }
            }
        }
        result
    }
    /// the length of the buffer.
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    /// checks if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}
impl Deref for IndexRefBuf {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

/// a reference to an auto updating index in a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexRef {
    ref_index: usize,
}

#[test]
pub fn make_sure_reference_points_to_same_element_after_modifications() {
    const INITIAL_BUF_SIZE: usize = 4;
    const INSERTED_SLICES_LEN: usize = 2;
    const SPLICE_SRC_LEN: usize = 2;
    const SPLICE_DST_LEN: usize = 3;
    for magic_index in 0..INITIAL_BUF_SIZE {
        // initialize the buffer with 1 magic element
        let mut raw_buf = vec![0u8; INITIAL_BUF_SIZE];
        raw_buf[magic_index] = 1;

        // convert to an index ref buffer
        let mut buf = IndexRefBuf::from_vec(raw_buf);

        // take a reference to the magic element
        let magic_elem_index_ref = buf.create_index_ref(magic_index);

        // insert elements at every possible location.
        // using steps of 2 because when inserting an element, we need to advance by 2, one for the actual element that we want
        // to skip, and one for the element that we just inserted, which we also want to skip.
        let len = buf.len();
        for i in (0..(len + 1) * 2).step_by(2) {
            buf.insert(i, 0);
        }

        // insert slices at every possible location.
        let slice_to_insert = [0u8; INSERTED_SLICES_LEN];
        let len = buf.len();
        let step = INSERTED_SLICES_LEN + 1;
        for i in (0..(len + 1) * step).step_by(step) {
            buf.insert_slice(i, slice_to_insert.as_slice());
        }

        // replace every slice of some source length with a slice of some destination length, unless it contains the magic element.
        let mut i = 0;
        let slice_to_splice = [0u8; SPLICE_DST_LEN];
        while i + SPLICE_SRC_LEN <= buf.len() {
            // if the slice contains the magic element, skip it.
            if buf[i..i + SPLICE_SRC_LEN].contains(&1) {
                i += 1;
                continue;
            }

            buf.splice(i..i + SPLICE_SRC_LEN, slice_to_splice.iter().copied());

            // skip the entire slice which we have just replaced.
            i += SPLICE_DST_LEN;
        }

        // now that we have finished messing with the buffer, make sure that our refernece still points to the magic element
        let final_index = buf.read_index_ref(magic_elem_index_ref);
        assert_eq!(buf[final_index], 1);
    }
}
