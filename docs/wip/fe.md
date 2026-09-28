# FE — féodalité et petites factions (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.

## État (2026-09-28)
- **Vagues 1 et 2 dans `main`** (26bea252) : F0-F5 + registres F4a-F4e. 91 factions (dont `fac_rebels`),
  186 provinces. Vérif complète verte (159 binaires cargo, 980 pytest, smoke).
- F5 : IA féodale (`FeudalPolicy` installée par `ai::feudal::install`, ADR 0110), ordres `DeclareCommise`,
  `GrantTitle`, `Revolt`, `SwitchAllegiance` ; doctrine « survie d'abord » des comtés.
- Intégration vague 2 : artefacts géo régénérés une fois sur les données fusionnées ; JSON partagés
  (maisons, portraits, front-end, index horizon, titres) fusionnés à trois voies ; `campaign.rs`
  compare désormais l'état aux données (plus de compteurs figés). Écus et bannières générés pour toutes
  les factions et maisons (62 factions n'en avaient pas) ; moteur héraldique : sautoir, clef, chaudière,
  main, nef, croissant, chef, bœuf. Pavie : bordure de jeu (doublon Palatinat).
- Tests recalés sur une graine (trajectoire changée par les nouvelles factions, pas de règle en cause) :
  `cv3_ai_stances` (graine 1), `m4` régence (graine 11), `c7_retinue` (Bohun au lieu de Lancastre),
  `g4` (allié ancre pris aux Pays-Bas).
- Tous les worktrees FE d'agents supprimés. `../game_project-fe` (`feat/fe`) = worktree d'intégration.

## Reprise
1. Vague 3 : F6 interface (`cent-ans-dev`), F7 portraits (`cent-ans-mech`, plafond 15 $, `--dry-run`
   d'abord), puis F8 équilibre (commise de Guyenne à régler : `commise.min_power_ratio`, `max_wars`).
- Difficultés des factions ajoutées (F4a-F4e) : choix éditoriaux des agents, à valider par le joueur.
- Incertitudes sourcées à confirmer : Burchard Grelle (Brême), Nicolas de Brno (Trente), Dietrich IX de
  Clèves, blasons gaéliques et italiens marqués `uncertain`.
- `virneburg` (maison) : « trois tours » non dessinées (écu plein, sans doublon).

## Points ouverts (à trancher en F5/F8)
- `ai/tests/g4.rs` (Brabant allié de l'Angleterre) en `#[ignore]` : un vassal peut-il s'allier hors de son suzerain ? (F5)
- Test 50 tours `m3_grid_ai` (`--ignored`) : rouge déjà sur main ; plus tôt avec les princes d'Empire vassaux (F8).
- Interprétations F3 à valider : conquête (§ 4.6), « plusieurs prétendants », lois de succession par titre (données à écrire).
- F6 : offres Protection/Arbitrage, ordre `ArbitratePrivateWar`, article `demand_title`, factions créées sans données d'affichage.
- F4a : Normandie en deux titres, héritiers manquants, couleurs héraldiques en doublon.
