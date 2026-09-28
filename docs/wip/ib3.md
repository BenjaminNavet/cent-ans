# IB3 — chaîne de bulles à Alt maintenu — fichier de reprise

Branche `feat/ib3-chain` (depuis `main` c1775c8a). Spec § 3.1-3.2, ADR 0109.
Fichiers : `game/scripts/codex/codex_bubbles.gd`, `game/tests/ib_chain_test.gd`,
`game/scripts/ui/shortcut_sheet.gd` (ligne Alt). `tooltip_style.json` (bloc `chain`) en lecture.

## État

- [x] Branche, dylib copiée, wip
- [ ] `codex_bubbles.gd` : réglages `chain` lus dans `tooltip_style.json`, état Alt, conversion,
  filles verrouillées à 0,12 s, remplacement de branche, surlignage de la source, grâce au relâché
- [ ] `ib_chain_test.gd` activé
- [ ] ligne Alt dans `shortcut_sheet.gd`
- [ ] fusion de `main`, smoke / ib_chain / p2c

## Prochaine étape

Implémenter dans `codex_bubbles.gd`.
