//! Module-specific engine Command arms that already exist (Wordbook / Recipes / SSH).
//!
//! **Freeze:** do not add new central Command/Event special-cases for the next module.
//! Prefer module `perform` + ports; grow helpers here only when touching an existing path.
//! Not a plugin ABI — opportunistic extract for navigability (see GOVERNANCE §2.7a).

mod recipes;
mod ssh;
mod wordbook;
