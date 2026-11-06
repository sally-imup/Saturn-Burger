//! Version metadata and category classification are planned for Phase 2.

/// Top-level categories shown by the future version browser.
///
/// This is a UI/domain outline only; manifest parsing and classification are not
/// implemented in Phase 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionCategory {
    Release,
    Snapshot,
    AprilFools,
    PreRelease,
    OldBeta,
    OldAlpha,
    Experimental,
}

impl VersionCategory {
    pub const ALL: [Self; 7] = [
        Self::Release,
        Self::Snapshot,
        Self::AprilFools,
        Self::PreRelease,
        Self::OldBeta,
        Self::OldAlpha,
        Self::Experimental,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Release => "Vanilla / Release",
            Self::Snapshot => "Snapshot",
            Self::AprilFools => "April Fools",
            Self::PreRelease => "Pre-release / RC",
            Self::OldBeta => "Old Beta",
            Self::OldAlpha => "Old Alpha",
            Self::Experimental => "Experimental / Custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VersionCategory;

    #[test]
    fn all_categories_have_non_empty_labels() {
        assert!(VersionCategory::ALL.iter().all(|category| !category.label().is_empty()));
    }

    #[test]
    fn category_list_contains_april_fools_separately() {
        assert!(VersionCategory::ALL.contains(&VersionCategory::AprilFools));
    }
}
