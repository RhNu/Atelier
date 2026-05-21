//! Shared foundation crate for Atelier.
//!
//! Keep this crate intentionally thin. Promote APIs here only after more than
//! one feature needs the same stable primitive or contract.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "atelier-foundation");
    }
}
