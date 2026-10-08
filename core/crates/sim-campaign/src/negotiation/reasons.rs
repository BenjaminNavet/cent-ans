//! The weighted reasons behind a verdict (French text, signed points).

use serde::{Deserialize, Serialize};

/// Reasons a recipient gives for accepting or refusing, in the order they
/// were weighed. Zero-point entries are never kept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReasonList(Vec<(String, i32)>);

impl ReasonList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a reason; a reason worth nothing says nothing and is dropped.
    pub fn push(&mut self, text: impl Into<String>, points: i32) {
        if points != 0 {
            self.0.push((text.into(), points));
        }
    }

    /// Adds the reason only when `condition` holds.
    pub fn push_if(&mut self, condition: bool, text: impl Into<String>, points: i32) {
        if condition {
            self.push(text, points);
        }
    }

    pub fn extend(&mut self, other: ReasonList) {
        self.0.extend(other.0);
    }

    pub fn total(&self) -> i32 {
        self.0.iter().map(|(_, points)| points).sum()
    }

    pub fn into_vec(self) -> Vec<(String, i32)> {
        self.0
    }

    /// The `count` heaviest objections, « raison (-12), … », for a refusal.
    pub fn heaviest_objections(&self, count: usize) -> String {
        let mut sorted: Vec<&(String, i32)> = self.0.iter().collect();
        sorted.sort_by_key(|(_, points)| *points);
        sorted
            .iter()
            .take(count)
            .map(|(text, points)| format!("{text} ({points:+})"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl std::ops::Deref for ReasonList {
    type Target = [(String, i32)];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> IntoIterator for &'a ReasonList {
    type Item = &'a (String, i32);
    type IntoIter = std::slice::Iter<'a, (String, i32)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for ReasonList {
    type Item = (String, i32);
    type IntoIter = std::vec::IntoIter<(String, i32)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<(String, i32)>> for ReasonList {
    fn from(list: Vec<(String, i32)>) -> Self {
        Self(list)
    }
}
