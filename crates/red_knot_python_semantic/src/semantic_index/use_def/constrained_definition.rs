use super::bitset::{BitSet, BitSetArray, BitSetIterator};
use ruff_index::newtype_index;

#[newtype_index]
pub(super) struct ScopedDefinitionId;

#[newtype_index]
pub(super) struct ScopedConstraintId;

/// Can reference this * 128 definitions efficiently; tune for performance vs memory.
const DEFINITION_BLOCKS: usize = 4;

type Definitions = BitSet<DEFINITION_BLOCKS>;

/// Can reference this * 128 constraints efficiently; tune for performance vs memory.
const CONSTRAINT_BLOCKS: usize = 4;

/// Can handle this many visible definitions per symbol at a given time efficiently.
const MAX_EXPECTED_VISIBLE_DEFINITIONS_PER_SYMBOL: usize = 16;

type Constraints = BitSetArray<CONSTRAINT_BLOCKS, MAX_EXPECTED_VISIBLE_DEFINITIONS_PER_SYMBOL>;

/// Constrained definitions visible for a symbol at a particular point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ConstrainedDefinitions {
    /// Which [`ScopedDefinitionId`] are visible?
    visible_definitions: Definitions,

    /// For each definition, which [`ScopedConstraintId`] apply?
    constraints: Constraints,

    /// Is unbound a visible definition as well?
    may_be_unbound: bool,
}

impl ConstrainedDefinitions {
    pub(super) fn unbound() -> Self {
        Self {
            visible_definitions: Definitions::default(),
            constraints: Constraints::default(),
            may_be_unbound: true,
        }
    }

    pub(super) fn with(definition_id: ScopedDefinitionId) -> Self {
        Self {
            visible_definitions: Definitions::with(definition_id.into()),
            constraints: Constraints::of_size(1),
            may_be_unbound: false,
        }
    }

    /// Add Unbound as a possible visible definition.
    pub(super) fn add_unbound(&mut self) {
        self.may_be_unbound = true;
    }

    /// Add given definition index as a visible definition
    pub(super) fn add_visible_definition(&mut self, definition_id: ScopedDefinitionId) {
        self.visible_definitions.insert(definition_id.as_u32());
        // TODO update constraints
    }

    /// Add given constraint index to all definitions
    pub(super) fn add_constraint(&mut self, constraint_id: ScopedConstraintId) {
        self.constraints.insert_in_each(constraint_id.into());
    }

    /// Merge another [`ConstrainedDefinitions`] into this one.
    pub(super) fn merge(&mut self, other: &ConstrainedDefinitions) {
        self.may_be_unbound |= other.may_be_unbound;
        self.visible_definitions.merge(&other.visible_definitions);
        // TODO merge constraints also
    }

    /// Get iterator over visible definitions.
    pub(super) fn iter_visible_definitions(&self) -> DefinitionIdIterator {
        DefinitionIdIterator {
            wrapped: self.visible_definitions.iter(),
        }
    }

    /// Is UNBOUND in the set of visible definitions?
    pub(super) fn may_be_unbound(&self) -> bool {
        self.may_be_unbound
    }
}

impl Default for ConstrainedDefinitions {
    fn default() -> Self {
        ConstrainedDefinitions::unbound()
    }
}

pub(super) struct DefinitionIdIterator<'a> {
    wrapped: BitSetIterator<'a, DEFINITION_BLOCKS>,
}

impl Iterator for DefinitionIdIterator<'_> {
    type Item = ScopedDefinitionId;

    fn next(&mut self) -> Option<Self::Item> {
        self.wrapped.next().map(ScopedDefinitionId::from_u32)
    }
}
