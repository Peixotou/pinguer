//! The nesting limit (ADR-0030).
//!
//! The limit bounds the depth of the tree, which every recursive pass over
//! it, and its destruction, recurse through. The levels that count are the
//! nodes of blocks, parenthesized expressions, prefix operators, calls,
//! member accesses, `as` conversions and operator chains.
//!
//! Blocks, parentheses and prefix operators are built top-down: the node
//! exists before what it contains is parsed, so [`Parser::enter`] counts
//! it while its contents are parsed. Calls, member accesses, conversions
//! and operator chains are built bottom-up: each new node encloses
//! everything built before it in the same chain, which becomes one level
//! deeper. For these, the parser measures the deepest level reached in the
//! chain and [`Parser::wrap`] adds a level to it, so that `f(x)()()` counts
//! the levels that the later calls add above `x`.

use std::mem;

use pinguer_diagnostics::{Diagnostic, codes};

use crate::{PResult, Parser, Reported};

/// The deepest nesting of counted constructs that the parser accepts.
pub const MAX_NESTING: u32 = 256;

impl Parser<'_> {
    /// Enters a construct built top-down, which the current token starts.
    /// The caller leaves it with [`Parser::leave`]; after an error,
    /// [`Parser::stmt`] and [`Parser::item`] restore the depth instead.
    pub(crate) fn enter(&mut self) -> PResult<()> {
        if self.depth == MAX_NESTING {
            return Err(self.nesting_too_deep());
        }
        self.depth += 1;
        self.deepest = self.deepest.max(self.depth);
        Ok(())
    }

    pub(crate) fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Starts measuring a chain at the current depth. Returns the
    /// measurement of the enclosing chain, which the caller passes to
    /// [`Parser::close_chain`] when its chain is complete.
    pub(crate) fn open_chain(&mut self) -> u32 {
        mem::replace(&mut self.deepest, self.depth)
    }

    /// Merges the measurement of a complete chain into the enclosing one,
    /// which contains it.
    pub(crate) fn close_chain(&mut self, enclosing: u32) {
        self.deepest = self.deepest.max(enclosing);
    }

    /// Accounts for a node, started by the current token, that encloses
    /// everything built so far in the chain. What the node contains besides
    /// that is parsed one level below the chain's own depth.
    pub(crate) fn wrap(&mut self) -> PResult<()> {
        let deepest = self.deepest.max(self.depth) + 1;
        if deepest > MAX_NESTING {
            return Err(self.nesting_too_deep());
        }
        self.deepest = deepest;
        Ok(())
    }

    fn nesting_too_deep(&mut self) -> Reported {
        let found = self.cursor.current();
        self.error(
            Diagnostic::error(format!(
                "code is nested more than {MAX_NESTING} levels deep"
            ))
            .with_code(codes::NESTING_TOO_DEEP)
            .with_primary(found.span, "this exceeds the nesting limit")
            .with_note(
                "blocks, parentheses, prefix operators, calls, member accesses, \
                 `as` conversions and operator chains each add a level",
            ),
        )
    }
}
