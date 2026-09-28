# IB5 — valeurs « avant → après » et état des prérequis (core + pont) — fichier de reprise

Branche `feat/ib5-live` (worktree agent). Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
§ 2.2 (effets) et § 4 (IB5), ADR 0109, orchestration `docs/wip/ib.md`. Cible cargo privée
`core/target-ib5` (à supprimer à la fin).

## Plan

- [x] 0. Squelette : `sim-campaign/src/preview.rs` (API vide)
- [ ] 1. Core : `effect_key`, prérequis (bâtiments, recrutement, techniques)
- [ ] 2. Core : avant → après (bâtiment : province ; technique : faction) sur copie de l'état
- [ ] 3. Pont : `before_after` et `requirements` dans `get_buildable`/`buildable`, `get_recruitable`, `get_tech_tree`
- [ ] 4. `rich_tooltip.gd` : lecture de `requirements` et de la clé d'effet ciblée
- [ ] 5. `ib_layout_test.gd` étendu ; tests de fin

## Choix

- Clé d'effet : `kind`, ou `kind:classe` / `kind:famille` pour un effet ciblé (évite de montrer la
  valeur d'une classe sur la ligne d'une autre).
- Prérequis : `live["requirements"]` = [{id, met}], `id` = entité requise ou `coastal`, `river`,
  `enabling_building`.

## Prochaine étape

Implémenter l'étape 1.
