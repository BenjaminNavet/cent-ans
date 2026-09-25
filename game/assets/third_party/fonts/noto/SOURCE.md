# Noto Serif et Noto Sans Symbols (sous-ensembles, polices de repli)

- **Auteur** : The Noto Project Authors
- **Licence** : SIL Open Font License 1.1 — voir `OFL.txt` (aucun nom de police réservé)
- **Pages** : https://fonts.google.com/noto/specimen/Noto+Serif et https://fonts.google.com/noto/specimen/Noto+Sans+Symbols
- **Téléchargement** : google/fonts, `ofl/notoserif/` et `ofl/notosanssymbols/`
- **Récupéré le** : 2026-09-25 (agent UI3)

Sous-ensembles faits avec `pyftsubset` (fontTools) :
- `NotoSerif-Currency.ttf` : U+20A0–20C0 (dont ₶, livre tournois, U+20B6, absent d'EB Garamond), U+2114, U+2116, U+2139 ;
- `NotoSansSymbols-Subset.ttf` : flèches, signes techniques, symboles divers et dingbats (U+2190–21FF,
  U+2300–23FF, U+2600–27BF, U+2B00–2BFF : ⚔ ✝ ⚒ ⚜ du journal et du rapport de saison).

Déclarées en polices de repli (`fallbacks`) d'EB Garamond dans `scenes/ui/parchment_theme.tres`.
