# IB5 — valeurs « avant → après » et état des prérequis (core + pont) — fichier de reprise

Branche `feat/ib5-live` (worktree agent). Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
§ 2.2 (effets) et § 4 (IB5), ADR 0109, orchestration `docs/wip/ib.md`. Cible cargo privée
`core/target-ib5` (à supprimer à la fin).

## Plan

- [x] 0. Squelette : `sim-campaign/src/preview.rs` (API vide)
- [x] 1. Core : `effect_key`, prérequis (bâtiments, recrutement, techniques)
- [x] 2. Core : avant → après (bâtiment : province ; technique : faction) sur copie de l'état
- [x] 3. Pont : `before_after` et `requirements` dans `get_province_city().buildable`,
  `settlement_buildable`, `get_recruitable` (prérequis seuls), `get_tech_tree`
  (`campaign_sim_preview.rs`)
- [x] 4. `rich_tooltip.gd` : `_requirement_met(live, id, repli)`, clé d'effet ciblée dans `effect_item`,
  l'effet vedette d'un bâtiment garde sa ligne quand il a un avant → après
- [x] 5. `ib_layout_test.gd` étendu (`_check_live`) ; `main` fusionné (conflit `buildings.rs` avec
  RS-C résolu : les deux fonctions gardées) ; `cargo test` sim-campaign + godot-bridge, smoke,
  `ib_layout_test`, `ib_chain_test`, `po_ui_test`, `p2c_ui_test` verts

## Choix

- Valeurs lues sur l'état réel et sur une **copie** où la chose est appliquée comme au tour
  (`buildings::complete_building`, extrait de `resolve_construction` ; insertion de la technique),
  avec les fonctions du tour. Coût mesuré (release) : 18 ms pour tous les bâtiments d'une cité,
  31 ms pour tout l'arbre des techniques.
- Statistiques : impôts/commerce/production → revenu de la province
  (`province_gross_income`, extrait de `faction_income_effective`) ou de la faction (technique) ;
  santé/richesse/biens/mécontentement → **valeur d'équilibre** de la jauge (`population::equilibrium`,
  point fixe de la mise à jour saisonnière ; `update_class` découpé en fonctions de cible partagées,
  arithmétique inchangée), moyenne pondérée par la population, ou une classe pour un effet ciblé ;
  points de recherche ; bâtiment seulement : fortifications, places de recrutement, résistance à la
  peste (%). Autres effets (piété, garnison, bataille, croissance…) : pas d'entrée.
- Clé d'effet : `kind`, ou `kind:classe` / `kind:famille` pour un effet ciblé ; une clé présente deux
  fois dans une même entité n'a pas d'entrée.
- Prérequis : `live["requirements"]` = [{id, met}], `id` = entité requise ou `coastal`, `river`,
  `enabling_building`.

## Prochaine étape

Lot terminé. Reste à l'orchestrateur : fusion dans `integration/ib` (dylib à reconstruire), jugement
visuel. Pistes non faites : avant → après des unités (aucun effet d'unité dans la spec), effets de
bataille et d'armée des techniques (pas de valeur unique hors d'une armée précise), cible
`core/target-ib5` supprimée.
