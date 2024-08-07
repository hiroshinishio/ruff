use crate::semantic_index::definition::Definition;
use crate::semantic_index::expression::Expression;
use crate::semantic_index::symbol::ScopedSymbolId;
use crate::types::Type;
use crate::Db;
use rustc_hash::FxHashMap;

/// Return type constraint, if any, on `definition` applied by `test`.
pub(crate) fn narrowing_constraint<'db>(
    db: &'db dyn Db,
    test: Expression<'db>,
    definition: Definition<'db>,
) -> Option<Type<'db>> {
    all_narrowing_constraints(db, test)
        .get(&definition.symbol(db))
        .copied()
}

#[salsa::tracked]
fn all_narrowing_constraints<'db>(
    db: &'db dyn Db,
    test: Expression<'db>,
) -> NarrowingConstraints<'db> {
    NarrowingConstraints::default()
}

type NarrowingConstraints<'db> = FxHashMap<ScopedSymbolId, Type<'db>>;
