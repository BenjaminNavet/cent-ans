# WIP — lot C4 (TW) : édits régionaux et chaînes de bâtiments

Mandat : `docs/design/2026-09-24-rapprochement-total-war.md` (C4), analyse TW §2.1
(« Bâtiments en chaînes/arbres », « Édits régionaux »). Démarré après la fusion
de la refonte colonies (`docs/wip/colonies.md`, ADR 0005).

## État

- [x] Repérage : `sim-campaign/src/buildings.rs` gère déjà `upgrades_from`
  (remplacement du niveau précédent à la complétion, `resolve_construction`) —
  la mécanique de chaîne existe déjà pour 3 familles (palissade→mur→château→
  bastion, muster field→armurerie, marché→foire). Le travail C4 étend les
  chaînes de données et ajoute les édits régionaux (mécanique neuve, calquée
  sur `table.rs`/H3 « La Table »).
- [x] Chaînes de données étendues : marché→maison des métiers→foire ;
  paroissiale→collégiale→{abbaye|cathédrale} (embranchement) ;
  moulin à vent→moulin à eau. Fortification et muster field→armurerie
  inchangées (déjà conformes).
- [x] Module `edicts.rs` (édits régionaux) + données `data/edicts/*.json`
  (6 édits dont `edict_none`) + schéma `edict.schema.json`.
- [x] `ProvinceState::edict`, `Order::SetEdict`/`OrderError::Edict`, IA
  `ai_choose_edicts` (câblée dans `ai_minimal.rs` et `ai::campaign`).
- [x] Pont Godot (`campaign_sim_edicts.rs` : `get_province_edict`,
  `get_edict_options`) + UI (`game/scripts/ui/edict_section.gd`, câblée dans
  `province_panel.gd` à côté de `TableSection` ; tri par catégorie/rang de
  `panel_widgets.gd::fill_buildable` pour lire les chaînes de bâtiments dans
  l'ordre de leurs paliers — le reste (rang, prérequis grisés, coût/durée)
  était déjà rendu par `RichTooltip.building`, sans changement).
- [x] Tests Rust : `core/crates/sim-campaign/tests/c4_chains.rs` (montée de
  niveau/remplacement/prérequis/embranchement/chaîne préexistante/données
  empilées d'avant C4) ; `edicts.rs` (10 tests unitaires : défaut, délai,
  refus doublon/inconnu/province partielle, effets fusionnés, IA
  déterministe, sauvegarde). `cargo test` plein vert (voir historique wip).
- [ ] Smoke Godot (import en cours) + captures `docs/img/c4-edits/`.
- [x] ADR 0011 (`docs/decisions/0011-edicts-and-building-chains.md`).

## Décisions

- Édits régionaux modélisés comme `table.rs`/H3 (bibliothèque de choix
  province, délai avant effet, IA déterministe partageant le score par livre).
  Différence : gratuits (pas de coût récurrent), un seul actif par province,
  condition = province entièrement contrôlée (`holds_whole_province`), sinon
  retour à `edict_none` (défaut, effets nuls).
- Effets d'édit injectés dans `EffectTotals` via le mécanisme existant
  (`CampaignState::province_effects` et `settlement_effects`) : aucune
  modification d'`economy.rs` nécessaire (déjà lu depuis `province_effects`).
- Chaînes : converties `bld_abbey`/`bld_cathedral`/`bld_guild_hall` de
  `required_building` (empilable) vers `upgrades_from` (remplace) pour former
  de vraies chaînes ; ajout `bld_church` (palier 2, chapelle→église) et mise à
  jour de `bld_water_mill` (palier 2, upgrade de `bld_windmill`).

## Prochaine étape

Voir section « État » ci-dessus, cocher au fur et à mesure.
