use super::bitset::{BitSet, BitSetArray, BitSetIterator};

/// Definition index zero is reserved for None (unbound).
pub(super) const UNBOUND: usize = 0;

/// Can reference this * 128 definitions efficiently; tune for performance vs memory.
const DEFINITION_BLOCKS: usize = 4;

type Definitions = BitSet<DEFINITION_BLOCKS>;

/// Can reference this * 128 constraints efficiently; tune for performance vs memory.
const CONSTRAINT_BLOCKS: usize = 4;

/// Can handle this many visible definitions per symbol at a given time efficiently.
const MAX_EXPECTED_VISIBLE_DEFINITIONS_PER_SYMBOL: usize = 16;

type Constraints = BitSetArray<CONSTRAINT_BLOCKS, MAX_EXPECTED_VISIBLE_DEFINITIONS_PER_SYMBOL>;

/// Constrained definitions visible for a symbol at a particular use (or end-of-scope).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ConstrainedDefinitions {
    /// Which indices in `all_definitions` are visible?
    visible_definitions: Definitions,

    /// For each definition, which constraints in `all_constraints` apply?
    constraints: Constraints,
}

impl ConstrainedDefinitions {
    pub(super) fn unbound() -> Self {
        Self::with(UNBOUND)
    }

    pub(super) fn with(definition_index: usize) -> Self {
        Self {
            visible_definitions: Definitions::with(definition_index),
            constraints: Constraints::of_size(1),
        }
    }

    /// Add given definition index as a visible definition
    pub(super) fn add_visible_definition(&mut self, definition_index: usize) {
        self.visible_definitions.insert(definition_index);
        // TODO update constraints
    }

    /// Add given constraint index to all definitions
    pub(super) fn add_constraint(&mut self, constraint_index: usize) {
        self.constraints.insert_in_each(constraint_index);
    }

    /// Merge another [`ConstrainedDefinitions`] into this one.
    pub(super) fn merge(&mut self, other: &ConstrainedDefinitions) {
        self.visible_definitions.merge(&other.visible_definitions);
        // TODO merge constraints also
    }

    /// Get iterator over visible definitions.
    pub(super) fn iter_visible_definitions(&self) -> BitSetIterator<DEFINITION_BLOCKS> {
        self.visible_definitions.iter()
    }

    /// Is UNBOUND in the set of visible definitions?
    pub(super) fn may_be_unbound(&self) -> bool {
        self.visible_definitions.contains(UNBOUND)
    }
}

impl Default for ConstrainedDefinitions {
    fn default() -> Self {
        ConstrainedDefinitions::unbound()
    }
}
