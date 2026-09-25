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
  change) ; filtre des ordres de l'IA dans `sim.rs::step` (ai.rs intact).
- Rendu : tuile d'horizon du site (`hist_<id>`), menu « Batailles historiques », entrée depuis la
  campagne (province + années du `campaign` de la carte).

## État
- [x] Squelette : schéma, test pytest, `historical.rs`, `sim/scenario.rs`, test Rust.
- [x] Outil `cent-ans geo battle-site` (`tools/cent_ans_tools/geo/battle_site.py`) : relief GLO-30
  (canopée WorldCover retirée, ouverture 90 m) écrit sur une ligne dans la carte, tuile d'horizon
  `hist_<id>` tournée dans le repère du champ, aperçu `docs/img/ep7/<id>_site.png`.
- [x] Crécy : site (crête Crécy-Wadicourt, vallée des Clercs, Maye, moulin, fosses), ordre de
  bataille, 4 vagues françaises en assaut ; Anglais 23/30 (test : 14 à 19 sur 20 graines).
- [x] Azincourt : entonnoir entre les bois d'Azincourt et de Tramecourt, labours détrempés, ailes
  de cavalerie ; Anglais 24/30.
- [x] Pont GDExtension (`historical_battles.rs` : `list_historical`, `setup_historical`,
  `get_historical`, `get_waves` ; site en campagne via `get_battle_setup` → `historical_site`).
- [x] Menu « Batailles historiques » (`historical_battles_menu.gd`), scène (`begin_historical`,
  ciel final + averse qui cesse, horizon du site, pas de phase de déploiement).
- [ ] Poitiers (Maupertuis, haies, vignes, chemin creux, Miosson).
- [ ] Essai Godot réel, captures `docs/img/ep7/`, ADR 0035, test Godot, vérifications finales.

## Décisions
- ai.rs n'est pas modifié (demande de coordination : SG5 y travaille). Les vagues françaises sont
  scriptées (`assault`) : les ordres de l'IA pour une vague retenue ou en assaut sont filtrés par
  `scenario_filter` (sim.rs::step) ; les Anglais « tiennent » leur poste (laisse).

## Prochaine étape
Poitiers, puis essai dans Godot et captures.
