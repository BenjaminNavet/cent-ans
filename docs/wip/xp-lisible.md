# Lot XP-LISIBLE (boucle juge 2)

Spec : `docs/audits/2026-10-29-experience.md` § « Lot XP-LISIBLE » ; constats M5, M6, M7.

## État
- [x] 1. Plancher typographique : `MIN_TEXT_PX = 13` (`ui/kit/style.rs`), borné dans `text_font`/`title_font`, littéraux corrigés, test source `client/tests/text_floor.rs`.
- [x] 3. Réglages : `subtitles`, `tooltip_delay`, `readable_font` (Atkinson Hyperlegible, OFL, `game/assets/third_party/fonts/atkinson/`), `text_size` appliqué à chaud (`apply_text_scale` réécrit les `TextFont` à l'écran).
- [ ] 4. Glossaire `data/ui/glossary.json` + schéma + test pytest.
- [ ] 2. Glyphes -> icônes à l'encre.
- [ ] 5. Raccourcis (K Codex, Recrutement, Ma faction, Féodalité ; F1).

## Prochaine étape
Glossaire, puis icônes, puis raccourcis ; ADR `docs/decisions/XXXX-xp-lisible-*.md`.
