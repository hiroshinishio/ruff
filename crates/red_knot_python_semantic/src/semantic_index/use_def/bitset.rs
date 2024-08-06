use std::collections::{btree_set, BTreeSet};

/// Ordered set of `usize`; bit-set for small values (up to 128 * B), BTreeSet for overflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BitSet<const B: usize> {
    /// Bit-set (in 128-bit blocks) for the first 128 * B entries.
    Blocks([u128; B]),

    /// Overflow beyond 128 * B.
    Overflow(BTreeSet<usize>),
}

impl<const B: usize> Default for BitSet<B> {
    fn default() -> Self {
        Self::Blocks([0; B])
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

    /// Convert from Blocks to Overflow representation.
    fn overflow(&mut self) {
        if matches!(self, Self::Blocks(_)) {
            let set = BTreeSet::from_iter(self.iter());
            *self = Self::Overflow(set);
        }
    }

    /// Insert a value into the BitSet.
    ///
    /// Return true if the value was newly inserted, false if already present.
    pub(crate) fn insert(&mut self, value: usize) -> bool {
        if value >= Self::BITS {
            self.overflow();
        }
        match self {
            Self::Blocks(blocks) => {
                let (block, index) = (value / 128, value % 128);
                let missing = blocks[block] & (1_u128 << index) == 0;
                blocks[block] |= 1_u128 << index;
                missing
            }
            Self::Overflow(set) => set.insert(value),
        }
    }

    /// Merge another BitSet into this one.
    ///
    /// Equivalent to (but often more efficient than) iterating the other BitSet and inserting its
    /// values one-by-one into this BitSet.
    pub(crate) fn merge(&mut self, other: &BitSet<B>) {
        match (self, other) {
            (Self::Blocks(myblocks), Self::Blocks(other_blocks)) => {
                for i in 0..B {
                    myblocks[i] |= other_blocks[i];
                }
            }
            (Self::Overflow(myset), Self::Overflow(other_set)) => {
                myset.extend(other_set);
            }
            (me, other) => {
                for value in other.iter() {
                    me.insert(value);
                }
            }
        }
    }

    /// Return `true` if this BitSet contains `value`; `false` if not.
    pub(crate) fn contains(&self, value: usize) -> bool {
        match self {
            Self::Blocks(blocks) => {
                let (block, index) = (value / 128, value % 128);
                blocks[block] & (1_u128 << index) != 0
            }
            Self::Overflow(set) => set.contains(&value),
        }
    }

    /// Return an iterator over the values (in ascending order) in this BitSet.
    pub(crate) fn iter(&self) -> BitSetIterator<'_, B> {
        match self {
            Self::Blocks(blocks) => BitSetIterator::Blocks(BitSetBlocksIterator {
                blocks: &blocks,
                cur_block_index: 0,
                cur_block: blocks[0],
            }),
            Self::Overflow(set) => BitSetIterator::Overflow(set.iter()),
        }
    }
}

/// Iterator over values in a [`BitSet`].
pub(crate) enum BitSetIterator<'a, const B: usize> {
    Blocks(BitSetBlocksIterator<'a, B>),
    Overflow(btree_set::Iter<'a, usize>),
}

pub(crate) struct BitSetBlocksIterator<'a, const B: usize> {
    /// The blocks we are iterating over.
    blocks: &'a [u128; B],

    /// The index of the block we are currently iterating through.
    cur_block_index: usize,

    /// The block we are currently iterating through (and zeroing as we go.)
    cur_block: u128,
}

impl<const B: usize> Iterator for BitSetIterator<'_, B> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Blocks(iter) => {
                while iter.cur_block == 0 {
                    if iter.cur_block_index == B - 1 {
                        return None;
                    }
                    iter.cur_block_index += 1;
                    iter.cur_block = iter.blocks[iter.cur_block_index];
                }
                let value = iter.cur_block.trailing_zeros() as usize;
                // reset the lowest set bit
                iter.cur_block &= iter.cur_block.wrapping_sub(1);
                Some(value + (128 * iter.cur_block_index))
            }
            Self::Overflow(set_iter) => set_iter.next().copied(),
        }
    }
}

impl<const B: usize> std::iter::FusedIterator for BitSetIterator<'_, B> {}

/// Array of BitSet<B>. Up to N stored inline, more than that in overflow vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BitSetArray<const B: usize, const N: usize> {
    Array {
        /// Array of N BitSets.
        array: [BitSet<B>; N],

        /// How many of the bitsets are used?
        size: usize,
    },

    Overflow(Vec<BitSet<B>>),
}

impl<const B: usize, const N: usize> Default for BitSetArray<B, N> {
    fn default() -> Self {
        Self::Array {
            array: std::array::from_fn(|_| BitSet::default()),
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

    fn overflow(&mut self) {
        match self {
            Self::Array { array, size } => {
                let mut vec: Vec<BitSet<B>> = vec![];
                for i in 0..(*size - 1) {
                    vec.push(array[i].clone());
                }
                *self = Self::Overflow(vec);
            }
            Self::Overflow(_) => {}
        }
    }

    /// Push an empty [`BitSet`] onto the end of the array.
    pub(crate) fn push_back(&mut self) {
        match self {
            Self::Array { array: _, size } => {
                *size += 1;
                if *size > N {
                    self.overflow();
                    self.push_back();
                }
            }
            Self::Overflow(vec) => vec.push(BitSet::default()),
        }
    }

    /// Insert `value` into every [`BitSet`] in this [`BitSetArray`].
    pub(crate) fn insert_in_each(&mut self, value: usize) {
        match self {
            Self::Array { array, size } => {
                for i in 0..*size {
                    array[i].insert(value);
                }
            }
            Self::Overflow(vec) => {
                for bitset in vec {
                    bitset.insert(value);
                }
            }
        }
    }

    /// Return an iterator over each [`BitSet`] in this [`BitSetArray`].
    pub(crate) fn iter(&self) -> BitSetArrayIterator<'_, B, N> {
        match self {
            Self::Array { array, size } => BitSetArrayIterator::Array {
                array,
                index: 0,
                size: *size,
            },
            Self::Overflow(vec) => BitSetArrayIterator::Overflow(vec.iter()),
        }
    }
}

/// Iterator over a [`BitSetArray`].
pub(crate) enum BitSetArrayIterator<'a, const B: usize, const N: usize> {
    Array {
        array: &'a [BitSet<B>; N],
        index: usize,
        size: usize,
    },

    Overflow(core::slice::Iter<'a, BitSet<B>>),
}

impl<'a, const B: usize, const N: usize> Iterator for BitSetArrayIterator<'a, B, N> {
    type Item = &'a BitSet<B>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Array { array, index, size } => {
                if *index >= *size {
                    return None;
                }
                let ret = Some(&array[*index]);
                *index += 1;
                ret
            }
            Self::Overflow(iter) => iter.next(),
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
            assert!(matches!(b, BitSet::Blocks(_)));
            assert_bitset(&b, &[3, 6, 27]);
        }

        #[test]
        fn iter_overflow() {
            let mut b = BitSet::<1>::with(140);
            b.insert(100);
            b.insert(129);
            assert!(matches!(b, BitSet::Overflow(_)));
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
            assert!(matches!(b, BitSet::Blocks(_)));
            assert_bitset(&b, &[45, 130]);
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

            assert!(matches!(ba, BitSetArray::Array { .. }));

            ba.push_back();
            assert!(matches!(ba, BitSetArray::Overflow(_)));
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
