//! Ink glyphs (port of game/scripts/ui/ink_glyph.gd): ink icons (`glyph_*` of
//! `data/ui/icons_ink.json`) instead of font glyphs (cross, check, swords, fleur-de-lis, flag,
//! hammer, letter, star) in texts. Each call takes a fallback: while the icon does not exist the
//! text keeps its original glyph.

use super::kit::icons::{IconFamily, IconLibrary};

/// True when `id` exists in the ink family.
pub fn has(library: &IconLibrary, id: &str) -> bool {
    library
        .get(id)
        .is_some_and(|entry| entry.family == IconFamily::Ink)
}

/// Rich-text `[img]` of the ink icon `id` at `size` px, else `fallback` unchanged.
pub fn bbcode(library: &IconLibrary, id: &str, fallback: &str, size: u32) -> String {
    match library.get(id) {
        Some(entry) if entry.family == IconFamily::Ink => {
            format!("[img={size}x{size}]{}[/img]", entry.path)
        }
        _ => fallback.to_owned(),
    }
}

/// Heading text `glyph  title`: the icon image when it exists, else the font glyph.
pub fn heading(library: &IconLibrary, id: &str, fallback: &str, title: &str, size: u32) -> String {
    format!("{}  {title}", bbcode(library, id, fallback, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_when_missing() {
        let library = IconLibrary::default();
        assert!(!has(&library, "glyph_cross"));
        assert_eq!(bbcode(&library, "glyph_cross", "✠", 16), "✠");
        assert_eq!(
            heading(&library, "glyph_cross", "✠", "Croisade", 20),
            "✠  Croisade"
        );
    }

    #[test]
    fn real_library_resolves_a_glyph_or_falls_back() {
        let theme_library = crate::paths::find_root().ok().map(|root| {
            IconLibrary::from_assets(|rel| {
                std::fs::read(crate::paths::asset_root(&root).join(rel)).ok()
            })
        });
        if let Some(library) = theme_library {
            let code = bbcode(&library, "glyph_cross", "✠", 16);
            assert!(code == "✠" || code.starts_with("[img=16x16]"));
        }
    }
}
