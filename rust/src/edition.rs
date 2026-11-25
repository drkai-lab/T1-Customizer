//! T1-Customizer is a single free edition: there is no paid gate and no feature split. This module only carries the product
//! identity strings so the rest of the code has one place to read them from.

/// Product name (Japanese).
pub const PRODUCT_NAME_JA: &str = "T1-Customizer";

/// Product name (English).
pub const PRODUCT_NAME_EN: &str = "T1-Customizer";

/// One-line `--help` description.
pub const CLI_ABOUT: &str =
    "T1-Customizer: T1 mini keyboard configuration tool (Linux / Windows / macOS)";

/// Every feature is available in the free edition.
pub fn is_unlocked() -> bool {
    true
}

/// T1 hardware features are part of the free edition.
pub fn t1_features_allowed() -> bool {
    true
}

/// Every GUI page is editable in the free edition.
pub fn page_editable(_page: usize) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_edition_is_fully_unlocked() {
        assert!(is_unlocked());
        assert!(t1_features_allowed());
        assert!(page_editable(0));
        assert!(page_editable(99));
    }
}
