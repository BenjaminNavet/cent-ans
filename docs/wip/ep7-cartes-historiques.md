# Lot EP7 — Cartes historiques : Crécy, Poitiers, Azincourt

Branche `feat/ep7-historical-maps` (worktree `.claude/worktrees/agent-aa22e819038559fe3`). Suivi du
chantier : `docs/wip/epic.md`. ADR : `docs/decisions/0035-cartes-historiques.md`.
Dépend d'EP1 (taille du champ), EP2 (horizon), EP3 (eau), EP6 (`DecorPlan`), EP8 (`set_start_hour`),
EP9 (fin de bataille).

## Conception
- Données : `data/battle_maps/<id>.json` (schéma `battle_map.schema.json`, test
  `tools/tests/test_battle_maps_schema.py`) : site (lon/lat, cap de l'axe d'attaque), champ, relief
  réel (grille de hauteurs cuite par `cent-ans geo battle-site`), bois, marais, ruisseaux, chemins,
  haies/fossés, décor EP6 (`DecorPlan`), ordres de bataille par blocs de régiments, vagues
  (« batailles » successives), régiments qui tiennent leur position, météo (et changements), heure.
- Cœur : `sim-battle/src/historical.rs` (types, `battle_setup`, `apply_site`, `start`) ;
  `src/sim/scenario.rs` (déploiement historique, vagues retenues, postes avec laisse, météo qui
  change) ; filtre des ordres de l'IA dans `sim.rs::step` ; `ai.rs::View::new` ignore les
  régiments d'une vague retenue (une ligne, hors des zones EP9b).
- Rendu : tuile d'horizon du site (`hist_<id>`), menu « Batailles historiques », entrée depuis la
  campagne (province + années du `campaign` de la carte).

## État
- [x] Squelette : schéma, test pytest, `historical.rs`, `sim/scenario.rs`, test Rust, Crécy stub.
- [ ] Outil `cent-ans geo battle-site` (relief GLO-30 + tuile d'horizon du site).
- [ ] Crécy complet (site, décor, ordre de bataille, équilibre ≥ 7/10).
- [ ] Pont GDExtension + menu « Batailles historiques » + scène.
- [ ] Entrée depuis la campagne.
- [ ] Azincourt, puis Poitiers.
- [ ] Captures `docs/img/ep7/`, ADR 0035, vérifications finales.

## Prochaine étape
Outil de cuisson du relief du site (GLO-30 dans `tools/geo/raw/copernicus30`, lien symbolique vers
le dépôt principal dans ce worktree).
