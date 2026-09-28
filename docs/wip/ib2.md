# IB2 — migration mécanique (tables → données, ~120 infobulles brutes) — fichier de reprise

Branche `feat/ib2-migrate` depuis `main` (05e711e1). Spec `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
§ 2.4, ADR 0109, orchestration `docs/wip/ib.md`. IB1/IB3/IB4 déjà dans `main`.

## Plan

1. Tables de `game/scripts/ui/rich_tooltip.gd` (`HUD_TEXTS`, `GAUGE_TEXTS`, `EFFECT_LABELS`,
   `STAT_LABELS`, + catégories/capacités/classes/branches) → `data/ui/tooltips.json`
   (fusion avec les `body` déjà posés par IB4), schéma étendu si besoin, `additionalProperties`
   conservé false. `RichTooltip` lit ces données avec repli (calque sur `CameraFeel`/`texts()`
   déjà existant côté IB4).
2. ~120 `tooltip_text = "…"` littéraux → `RichTooltip.plain(title, body, hint)` (spec
   `kind: "plain"`) ; textes dans `data/ui/tooltips.json` bloc `plain`.
3. `unit_card.gd` : déjà migré par IB1, vérification seulement.
4. Tests : `ib_plain_test.gd` (aucun `tooltip_text` littéral hors exceptions, titre présent),
   pytest `test_tooltip_schemas.py` (clés `plain` référencées existent).

## État

- [ ] Squelette (ce fichier, branche, dylib copiée, import fait)
- [ ] 1. Tables → `tooltips.json` + schéma
- [ ] 2. `RichTooltip.plain` + mécanisme d'attache (`attach_plain` ou repli sur `panel_for`)
- [ ] 3. Migration des ~120 littéraux, par lots de fichiers (carte, bataille, menus)
- [ ] 4. Tests

## Choix à trancher pendant le travail

- Mécanisme d'attache pour les contrôles natifs (`Button`/`Control` hors `RichButton`/`IconChip`/
  `RichPanel`) : à choisir après lecture de `panel_for` (IB1) — probablement un
  `RichTooltip.attach_plain(control, key, title, body, hint)` qui pose la clé `ib:plain:<key>`
  dans `tooltip_text` + éventuel texte dynamique en métadonnée, et une entrée dans `panel_for`/
  `spec_for` pour `kind == "plain"`.

## Journal

- 28/09 : lecture spec/ADR/wip IB, `rich_tooltip.gd` (1183 lignes), `tooltips.json`, schéma,
  `camera_feel.gd` (patron de repli). Branche créée depuis `main` (05e711e1), dylib copiée depuis
  `../../game/bin`, import headless OK. 131 `tooltip_text = "..."` littéraux recensés par grep.
  Prochaine étape : lot 1 (tables → données).
