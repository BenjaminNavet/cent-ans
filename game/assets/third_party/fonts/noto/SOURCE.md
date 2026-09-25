# Noto Serif, Noto Sans Symbols et Noto Sans Symbols 2 (sous-ensembles, polices de repli)

- **Auteur** : The Noto Project Authors
- **Licence** : SIL Open Font License 1.1 — voir `OFL.txt` (aucun nom de police réservé)
- **Pages** : https://fonts.google.com/noto/specimen/Noto+Serif et https://fonts.google.com/noto/specimen/Noto+Sans+Symbols
- **Téléchargement** : google/fonts, `ofl/notoserif/` et `ofl/notosanssymbols/`
- **Récupéré le** : 2026-09-25 (agent UI3)

Sous-ensembles faits avec `pyftsubset` (fontTools) :
- `NotoSerif-Currency.ttf` : U+20A0–20C0 (dont ₶, livre tournois, U+20B6, absent d'EB Garamond), U+2114, U+2116, U+2139 ;
- `NotoSansSymbols-Subset.ttf` : flèches, signes techniques, symboles divers et dingbats (U+2190–21FF,
  U+2300–23FF, U+2600–27BF, U+2B00–2BFF : ⚔ ✝ ⚒ ⚜ du journal et du rapport de saison) ;
- `NotoSansSymbols2-Subset.ttf` (`ofl/notosanssymbols2/`) : mêmes plages plus les formes géométriques
  U+25A0–25FF (✉ ✦ ✧ ⌖ ⌛ ♔ ♛ ☠ ⚠ ★ ♨ ▲ ■ ▼ absents des deux autres).

Déclarées en polices de repli (`fallbacks`) d'EB Garamond dans `scenes/ui/parchment_theme.tres`.

Métriques verticales (lot UI1, 2026-09-25) : `hhea` et `OS/2` (ascender/descender, win et typo)
ramenées à celles d'EB Garamond (1007 / -298 pour 1000 unités) avec fontTools. Sans cela, Godot
prenait la hauteur maximale de la chaîne de repli : chaque ligne du thème faisait 36 px au lieu
de 24 px (corps 17). Les glyphes ne sont pas modifiés.
