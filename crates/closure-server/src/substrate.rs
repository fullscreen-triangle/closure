//! Why the substrate is generated, and what is *not* generated with it.
//!
//! This module used to hold Zürich: thirty-one positions, six named regions,
//! hand-placed. The city moved to [`crate::society`], which draws one per
//! session. What stayed here is the argument for why that was always the
//! right shape, and the one function the argument is about.
//!
//! ## Why there is no population
//!
//! There was a list of modules a voice could be matched against. Matching a
//! voice to an entry in a list is retrieval whatever the entries are called,
//! and Theorem 4.3 says no operation has that signature. What replaced it is
//! not a better list: it is the absence of one. A voice is a split of the
//! character talking in its region, and the character behind a voice is built
//! by capping and amalgamating those moderators — from where the voice
//! actually spoke, never from a table.
//!
//! Generating the society does not weaken this. A generated catalogue would
//! be a catalogue. What is generated is a *graph and its regions*, and the
//! characters are still cut from it by [`closure_runtime::moderator`] rather
//! than drawn.
//!
//! ## Why the bodies are templates
//!
//! The utterance text is generated from the voice handle and the region.
//! That is deliberately thin: nothing in the runtime reads a post's body —
//! not visibility, not the act classifier, not the feed order. Text that
//! carried meaning the mechanism then acted on would be a fifth operation
//! (§11), and there is no fifth. When a language model is attached later it
//! replaces this function and nothing else, because nothing else consumes
//! what it produces.
//!
//! That constraint is why a generated society is not a poorer substrate than
//! a researched one. Nothing downstream can tell the difference, because
//! nothing downstream reads anything but reach.

pub use crate::society::utterance_body;
