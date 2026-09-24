# M3 — tour séquentiel et IA sur la grille (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 3.4, § 4, § 7. ADR : `docs/decisions/0010-free-army-movement.md`.
Branche : `m3-turn-ai` (worktree `agent-a2eb8f7f4bd84c3dd`), depuis `main` 93d466c (M1 + M2).

## État

- [x] Squelette : `data/ai/grid.json` + schéma `ai_grid.schema.json` + `AiGrid` (data-model, `GameData::ai_grid`) + test pytest.
- [x] Tour séquentiel (`turn.rs`) : `end_turn_with` = batailles du joueur encore en attente auto-résolues, puis
  `play_ai_turn` pour chaque IA par id (marches reprises, planificateur, ordres exécutés un par un), puis
  `resolve_end_of_turn` (phases § 1.3 sans mouvement, nouvelle saison, points remis à neuf, reprise des
  marches du joueur). `end_turn_profiled` rend aussi le temps de chaque IA.
- [x] Batailles IA contre joueur pendant le tour de l'IA (`CampaignState::ai_turn`, jamais sauvegardé) :
  auto-résolues (bataille de campagne et assaut), avec un événement « Pendant le tour de… » adressé au joueur.
  Les batailles que le joueur engage pendant son tour restent en 3D (`pending_battles`).
- [x] IA grille (`ai/src/grid.rs`, `GridPlanner`) :
  - Dijkstra du graphe des colonies mis en cache pour le tour de la faction (clé : départ, budget, plafond,
    armées évitées), avec les places ennemies et les places occupées par l'ennemi précalculées ;
  - évitement : les colonies à moins de `avoid_radius_km` (12) d'une armée ennemie plus forte que
    `avoid_ratio` × la puissance de l'armée sont des terminus (on peut y aller, jamais passer par elles) ;
  - `Attack` sur l'armée ennemie la plus proche dans `attack_reach_share` (0,8) des km restants quand la
    puissance de notre camp au point de contact dépasse `attack_ratio` (1,5, le seuil `SIEGE_SUPERIORITY` /
    `ATTACK_SUPERIORITY` existant) × la leur ; pas contre une armée derrière les murs d'une place ennemie
    (le siège s'en charge) ; pas pour une armée brisée ou qui assiège ;
  - exécution : un `MoveArmy` par étape du graphe (jusqu'à la première au-delà de la portée de la saison,
    `max_legs_per_turn` = 6 au plus), donc des A* courts ; traversée maritime : marche jusqu'au port puis
    `Embark` quand l'armée y est avec une saison entière.
- [x] `ai_minimal` inchangé (ni attaque ni mer, documenté).
- [x] Tests : `sim-campaign/tests/m3_sequential_turn.rs` (ordre des IA, bataille IA contre joueur
  auto-résolue et annoncée, attaque du joueur laissée en 3D, reprise de la marche du joueur),
  `ai/tests/m3_grid_ai.rs` (attaque, pas d'attaque contre plus fort, évitement, embarquement à Douvres,
  déterminisme, performance, 50 tours × 8 graines en `--ignored`).
- [x] Sondes : `examples/turn_perf.rs` (meilleur de n par faction et par tour), `settlements_probe` compte
  batailles et débarquements.
- [ ] Fusion de main (C4/C5 : garder `resolve_trade` et `ai_choose_edicts`), fmt/clippy/test, build.sh, smoke.

## Mesures (`settlements_probe 50 1..8`, release)

| Indicateur | avant M2 (1d199c4) | M2 (93d466c) | M3 |
|---|---|---|---|
| Trésor final FR / EN | 96 240 / 18 326 | 98 872 / 19 212 | 110 978 / 14 450 |
| Δ provinces FR / EN | −0,4 / −0,1 | +0,9 / −0,5 | +0,0 / −0,2 |
| Δ colonies FR / EN | −1,8 / −0,9 | +3,0 / −3,1 | +0,2 / −2,8 |
| Écosse Δ colonies ; banqueroutes (8 graines) | −0,6 ; 8 | −3,6 ; 0 | −1,2 ; 15 (12 dans la graine 7) |
| Sièges / tour (cités / autres) | 0,8 / 3,1 | 1,5 / 5,9 | 0,6 / 3,0 |
| Prises / graine (cités / secondaires) | 10,1 / 65,8 | 10,6 / 93,5 | 8,2 / 55,0 |
| Batailles / graine | 58,9 | **0** | 79,2 |
| Débarquements en terre hostile / graine (Angleterre) | 3,5 (3,4) | — | 3,4 (3,2) |
| Armées bloquées / tour (niv. 4) | 0,3 (0,1) | 0,7 (0,5) | 0,1 (0,1) |
| Débandades / graine | 1,2 | 0 | 0 |
| s / graine | 5,8-6,8 | 15,4 | 6,4-8,0 |

M2 : plus aucune bataille de campagne (les marches s'arrêtent dans la zone de contrôle, l'IA n'émet pas
`Attack`) et l'Angleterre ne traversait plus la mer. M3 rétablit les deux.

Performance (`turn_perf 50 3 1`, release, meilleur de 3, machine chargée) : moyenne 2,4 ms, médiane 1,5 ms,
p95 7,7 ms, p99 14,6 ms, max 30 ms par faction et par tour (28 IA) ; la planification pèse ~75-80 %.

## Limites

- L'évitement porte sur les colonies du graphe ; le segment de grille entre deux colonies peut encore frôler
  une zone de contrôle (la marche s'y arrête alors, règle M2).
- Débandades : la règle de repli sur la grille (M2) trouve presque toujours un recul de 15 km, d'où 0
  débandade ; à revoir en M5.
- Écosse toujours fragile (trésor négatif 8/8 graines, comme avant M2 7/8).

## Prochaine étape

Fusion de main juste avant de rendre, puis build.sh et smoke Godot.
