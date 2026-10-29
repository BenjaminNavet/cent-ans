# Lot XP-LISIBLE (boucle juge 2)

Spec : `docs/audits/2026-10-29-experience.md` § « Lot XP-LISIBLE » ; constats M5, M6, M7. ADR 0801.

## État
- [x] 1. Plancher typographique : `MIN_TEXT_PX = 13` (`ui/kit/style.rs`), borné dans `text_font`/`title_font`, littéraux corrigés, test source `client/tests/text_floor.rs`.
- [x] 3. Réglages : `subtitles`, `tooltip_delay`, `readable_font` (Atkinson Hyperlegible, OFL, `game/assets/third_party/fonts/atkinson/`), `text_size` appliqué à chaud (`apply_text_scale` réécrit les `TextFont` à l'écran).
- [x] 4. Glossaire `data/ui/glossary.json` + schéma + `tools/tests/test_ui_glossary.py` ; « agitation » et « colonie » corrigés (missions.rs, difficulty.rs, advisor.json, agents.json, tooltips, codex).
- [~] 2. Icônes : 7 glyphes à l'encre (`tools/make_ink_glyphs.py`), alias epees/etoile/croix, composant `InkGlyph` ; migrés : case à cocher, objectifs. Reste : ▲▼ ✓✗ ★ ⚔ des tooltips et du HUD de bataille.
- [x] 5. Raccourcis : K lu dans le keymap, Maj+R Recrutement, P Ma faction, Maj+F Féodalité ; B, G, L retirés de F1 et du keymap.

## Prochaine étape
Migrer les glyphes restants vers `InkGlyph` / `ink_glyph::bbcode` ; vérifier fumée campaign/battle (dump `ui_kit.icons.unresolved_on_screen`).
