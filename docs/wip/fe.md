# FE — féodalité et petites factions (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.

## État (2026-09-28)
- **Vague 1 dans `main`** (a2672d2e) : F0-F4a + réconciliation F1/F2/F3 (cdad6965) + données dérivées F4a
  (colonies, horizon, navgrid, héraldique, front-end). Vérif complète verte (155 binaires cargo, 896 pytest, smoke).
  Retouches d'intégration : `cv3_ai_stances` déterminisme sur la graine 1 (la 7 a perdu son embuscade avec
  les fiefs français) ; Sées : collégiale (une ville ne porte pas de cathédrale).
- Worktrees vague 1 supprimés. `../game_project-fe` (`feat/fe`) reste le worktree d'intégration.
- **Vague 2 en cours** (depuis `main` a2672d2e ou `feat/fe` a9f7b409 pour F5) :
  - F5 IA féodale : `../game_project-fe5` (`feat/fe5-ia`) ;
  - F4b Empire et Pays-Bas : `../game_project-fe4b` (`feat/fe4b-empire`) ;
  - F4c îles Britanniques : `../game_project-fe4c` (`feat/fe4c-iles`) ;
  - F4d Ibérie et F4e Italie : à lancer quand des places d'agents se libèrent (plafond 10 sur le dépôt).

## Reprise
1. Fusionner chaque lot dans `feat/fe`. Artefacts géo GLOBAUX (`provinces.geojson`, rasters, `data/map/*`,
   `navgrid.png`, aperçus) : prendre un côté puis relancer le pipeline sur les données fusionnées
   (`cent-ans geo provinces`, `geo settlements`, `geo hamlets`, `geo horizon --province …`, `geo navgrid`).
2. Vérif complète, ff `main`, puis vague 3 (F6 UI, F7 portraits, F8 équilibre).
- Difficultés (2-3) des 7 factions F4a : choix éditorial de l'agent, à valider par le joueur.

## Points ouverts (à trancher en F5/F8)
- `ai/tests/g4.rs` (Brabant allié de l'Angleterre) en `#[ignore]` : un vassal peut-il s'allier hors de son suzerain ? (F5)
- Test 50 tours `m3_grid_ai` (`--ignored`) : rouge déjà sur main ; plus tôt avec les princes d'Empire vassaux (F8).
- Interprétations F3 à valider : conquête (§ 4.6), « plusieurs prétendants », lois de succession par titre (données à écrire).
- F6 : offres Protection/Arbitrage, ordre `ArbitratePrivateWar`, article `demand_title`, factions créées sans données d'affichage.
- F4a : Normandie en deux titres, héritiers manquants, couleurs héraldiques en doublon.
