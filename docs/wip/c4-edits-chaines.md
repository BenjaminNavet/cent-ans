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
- [ ] Chaînes de données étendues (marché→halle→foire, chapelle→église→
  abbaye/cathédrale, moulin→moulin à eau).
- [ ] Module `edicts.rs` (édits régionaux) + données `data/edicts/*.json` +
  schéma.
- [ ] `ProvinceState::edict`, `Order::SetEdict`, IA `ai_choose_edicts`.
- [ ] Pont Godot (`campaign_sim_edicts.rs`) + UI (panneau construction : arbre ;
  panneau province : sélecteur d'édit).
- [ ] Tests (montée niveau, remplacement, prérequis, édits, IA, ancienne
  sauvegarde) + smoke.
- [ ] ADR 0010.

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
