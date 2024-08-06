use std::collections::{btree_set, BTreeSet};

/// Ordered set of `usize`; bit-set for small values (up to 128 * B), BTreeSet for overflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BitSet<const B: usize> {
    /// Bit-set (in 128-bit blocks) for the first 128 * B entries.
    blocks: [u128; B],

    /// Overflow storage for entries beyond 128 * B.
    overflow: BTreeSet<usize>,
}

impl<const B: usize> Default for BitSet<B> {
    fn default() -> Self {
        Self {
            blocks: [0; B],
            overflow: BTreeSet::new(),
        }
    }
}

impl<const B: usize> BitSet<B> {
    const BITS: usize = 128 * B;

    /// Create and return a new BitSet with a single `value` inserted.
    pub(crate) fn with(value: usize) -> Self {
        let mut bitset = Self::default();
        bitset.insert(value);
        bitset
    }

    /// Insert a value into the BitSet.
    ///
    /// Return true if the value was newly inserted, false if already present.
    pub(crate) fn insert(&mut self, value: usize) -> bool {
        if value >= Self::BITS {
            self.overflow.insert(value)
        } else {
            let (block, index) = (value / 128, value % 128);
            let missing = self.blocks[block] & (1_u128 << index) == 0;
            self.blocks[block] |= 1_u128 << index;
            missing
        }
    }

    /// Merge another BitSet into this one.
    ///
    /// Equivalent to (but more efficient than) iterating the other BitSet and inserting its values
    /// one-by-one into this BitSet.
    pub(crate) fn merge(&mut self, other: &BitSet<B>) {
        for i in 0..B {
            self.blocks[i] |= other.blocks[i];
        }
        self.overflow.extend(&other.overflow);
    }

    /// Return `true` if this BitSet contains `value`; `false` if not.
    pub(crate) fn contains(&self, value: usize) -> bool {
        if value >= Self::BITS {
            self.overflow.contains(&value)
        } else {
            let (block, index) = (value / 128, value % 128);
            self.blocks[block] & (1_u128 << index) != 0
        }
    }

    /// Return an iterator over the values (in ascending order) in this BitSet.
    pub(crate) fn iter(&self) -> BitSetIterator<'_, B> {
        BitSetIterator {
            bitset: self,
            cur_block_index: 0,
            cur_block: self.blocks[0],
            overflow_iterator: None,
        }
    }
}

/// Iterator over values in a [`BitSet`].
pub(crate) struct BitSetIterator<'a, const B: usize> {
    /// The [`BitSet`] we are iterating over.
    bitset: &'a BitSet<B>,

    /// The index of the block we are currently iterating through.
    cur_block_index: usize,

    /// The block we are currently iterating through (and zeroing as we go.)
    cur_block: u128,

    /// An iterator through the overflow [`BTreeSet`], if
    overflow_iterator: Option<btree_set::Iter<'a, usize>>,
}

impl<const B: usize> Iterator for BitSetIterator<'_, B> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cur_block == 0 {
            if self.cur_block_index == B - 1 {
                if let Some(iter) = &mut self.overflow_iterator {
                    return iter.next().copied();
                } else {
                    if self.bitset.overflow.is_empty() {
                        return None;
                    }
                    self.overflow_iterator = Some(self.bitset.overflow.iter());
                    return self.overflow_iterator.as_mut().unwrap().next().copied();
                }
            }
            self.cur_block_index += 1;
            self.cur_block = self.bitset.blocks[self.cur_block_index];
        }
        let value = self.cur_block.trailing_zeros() as usize;
        // reset the lowest set bit
        self.cur_block &= self.cur_block.wrapping_sub(1);
        Some(value + (128 * self.cur_block_index))
    }
}

impl<const B: usize> std::iter::FusedIterator for BitSetIterator<'_, B> {}

/// Array of BitSet<B>. Up to N stored inline, more than that in overflow vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BitSetArray<const B: usize, const N: usize> {
    /// Array of first N BitSets.
    array: [BitSet<B>; N],

    /// Overflow storage for BitSets beyond N.
    overflow: Vec<BitSet<B>>,

    /// How many of the bitsets are used?
    size: usize,
}

impl<const B: usize, const N: usize> Default for BitSetArray<B, N> {
    fn default() -> Self {
        Self {
            array: std::array::from_fn(|_| BitSet::default()),
            overflow: vec![],
            size: 0,
        }
    }
}

impl<const B: usize, const N: usize> BitSetArray<B, N> {
    /// Create a [`BitSetArray`] of `size` empty [`BitSet`]s.
    pub(crate) fn of_size(size: usize) -> Self {
        let mut array = Self::default();
        for _ in 0..size {
            array.push_back();
        }
        array
    }

    /// Push an empty [`BitSet`] onto the end of the array and return a reference to it.
    pub(crate) fn push_back(&mut self) -> &BitSet<B> {
        self.size += 1;
        if self.size > N {
            self.overflow.push(BitSet::<B>::default());
            self.overflow.last().unwrap()
        } else {
            &self.array[self.size - 1]
        }
    }

    /// Push an empty [`BitSet`] onto the front of the array and return a reference to it.
    pub(crate) fn push_front(&mut self) -> &BitSet<B> {
        self.size += 1;
        if self.size > N {
            self.overflow.push(BitSet::<B>::default());
            self.overflow.last().unwrap()
        } else {
            &self.array[self.size - 1]
        }
    }

    /// Insert `value` into every [`BitSet`] in this [`BitSetArray`].
    pub(crate) fn insert_in_each(&mut self, value: usize) {
        let mut inserted = 0;
        for bitset in &mut self.array {
            if inserted >= self.size {
                return;
            }
            bitset.insert(value);
            inserted += 1;
        }
        for bitset in &mut self.overflow {
            bitset.insert(value);
        }
    }

    /// Return an iterator over each [`BitSet`] in this [`BitSetArray`].
    pub(crate) fn iter(&self) -> BitSetArrayIterator<'_, B, N> {
        BitSetArrayIterator {
            array: self,
            wrapped: self.array.iter(),
            in_overflow: false,
            yielded: 0,
        }
    }
}

/// Iterator over a [`BitSetArray`].
pub(crate) struct BitSetArrayIterator<'a, const B: usize, const N: usize> {
    /// The [`BitSetArray`] we are iterating over.
    array: &'a BitSetArray<B, N>,

    /// Internal iterator over either the inline array or the overflow vector.
    wrapped: core::slice::Iter<'a, BitSet<B>>,

    /// `true` if we are now iterating over the overflow vector, otherwise `false`.
    in_overflow: bool,

    /// The number of [`BitSet`] we have yielded so far.
    yielded: usize,
}

impl<'a, const B: usize, const N: usize> Iterator for BitSetArrayIterator<'a, B, N> {
    type Item = &'a BitSet<B>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.yielded >= self.array.size {
            return None;
        }
        self.yielded += 1;
        if self.in_overflow {
            self.wrapped.next()
        } else {
            let ret = self.wrapped.next();
            if let Some(val) = ret {
                Some(val)
            } else {
                self.wrapped = self.array.overflow.iter();
                self.in_overflow = true;
                self.wrapped.next()
            }
        }
    }
}

impl<const B: usize, const N: usize> std::iter::FusedIterator for BitSetArrayIterator<'_, B, N> {}

#[cfg(test)]
mod tests {
    use super::{BitSet, BitSetArray};

    fn assert_bitset<const B: usize>(bitset: &BitSet<B>, contents: &[usize]) {
        assert_eq!(bitset.iter().collect::<Vec<_>>(), contents);
    }

    mod bitset {
        use super::{assert_bitset, BitSet};

        #[test]
        fn iter() {
            let mut b = BitSet::<1>::with(3);
            b.insert(27);
            b.insert(6);
            assert_bitset(&b, &[3, 6, 27]);
        }

        #[test]
        fn iter_overflow() {
            let mut b = BitSet::<1>::with(140);
            b.insert(100);
            b.insert(129);
            assert_eq!(
                b.overflow.iter().copied().collect::<Vec<usize>>(),
                vec![129, 140]
            );
            assert_bitset(&b, &[100, 129, 140]);
        }

        #[test]
        fn merge() {
            let mut b1 = BitSet::<1>::with(4);
            let mut b2 = BitSet::<1>::with(21);
            b1.insert(179);
            b2.insert(130);
            b2.insert(179);
            b1.merge(&b2);
            assert_bitset(&b1, &[4, 21, 130, 179]);
        }

        #[test]
        fn multiple_blocks() {
            let mut b = BitSet::<2>::with(130);
            b.insert(45);
            b.insert(280);
            assert_eq!(
                b.overflow.iter().copied().collect::<Vec<usize>>(),
                vec![280]
            );
            assert_bitset(&b, &[45, 130, 280]);
        }

        #[test]
        fn contains() {
            let b = BitSet::<1>::with(5);
            assert!(b.contains(5));
            assert!(!b.contains(4));
        }
    }

    fn assert_array<const B: usize, const N: usize>(
        array: &BitSetArray<B, N>,
        contents: &[Vec<usize>],
    ) {
        assert_eq!(
            array
                .iter()
                .map(|bitset| bitset.iter().collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            contents
        );
    }

    mod bitset_array {
        use super::{assert_array, BitSetArray};

        #[test]
        fn insert_in_each() {
            let mut ba = BitSetArray::<1, 2>::default();
            assert_array(&ba, &[]);

            ba.push_back();
            assert_array(&ba, &[vec![]]);

            ba.insert_in_each(3);
            assert_array(&ba, &[vec![3]]);

            ba.push_back();
            assert_array(&ba, &[vec![3], vec![]]);

            ba.insert_in_each(79);
            assert_array(&ba, &[vec![3, 79], vec![79]]);

            ba.push_back();
            assert_array(&ba, &[vec![3, 79], vec![79], vec![]]);

            ba.insert_in_each(130);
            assert_array(&ba, &[vec![3, 79, 130], vec![79, 130], vec![130]]);
        }

        #[test]
        fn of_size() {
            let mut ba = BitSetArray::<1, 2>::of_size(1);
            ba.insert_in_each(5);
            assert_array(&ba, &[vec![5]])
        }
    }
}
