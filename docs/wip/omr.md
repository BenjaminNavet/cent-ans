# OMR — restes de la carte Oural–Méditerranée (orchestration)

Suite de `docs/wip/om.md` (section « Reste »), spec `docs/superpowers/specs/2026-09-28-oural-mediterranee-design.md`.
Mandat du joueur (2026-09-29) : traiter les quatre groupes (équilibre Est, perf, relecture
historienne, contenu Est), lots enchaînés sans redemander.

## Contraintes
- Budget : total consigné 55,95 $ (> plafond 50 $ v1) ; seule dépense autorisée : le reste de
  l'enveloppe OM (1,95 $). Tout le reste en génération locale / sources libres.
- Disque : 44 Go libres au départ. Cargo : cible partagée `core/target` du checkout principal
  avec un profil par lot (`--profile rN`, debug=0), supprimé après fusion. Au plus 3 lots qui
  compilent en même temps. Arrêt d'un lot si `df` < 15 Go.
- Ne jamais lancer `q3_playtest.gd` ni un pilote fenêtré (il réécrit `user://settings.cfg`).

## Lots (worktree `../gp-omr-rN`, branche `feat/omr-rN`, note `docs/wip/omr-rN.md`)
| Lot | Objet | Agent |
|---|---|---|
| R1 | Coût IA par tour (GridPlanner, agent_dijkstra, attitude, faction_power) : cible ≤ ×2 main | cent-ans-dev |
| R2 | Chargement 11,4 s (relief_landcover 7,8 s) et RSS 2,9 Go (relief_shade tuilé/compressé) | cent-ans-dev |
| R3 | Équilibre Est : commise de Guyenne au tour 1, banqueroutes et révoltes des petites factions, sonde 464 t. × 10 graines | cent-ans-dev |
| R4 | Relecture historienne : 31 objectifs I1, incertitudes om-d1..d6, colonies ramenées de loin | cent-ans-dev |
| R5 | Unités orientales (mamelouks, archers montés des steppes, akıncı, druzhina…) par culture | cent-ans-dev |
| R6 | Musique orthodoxe / orientale (sources libres) + retouche portraits Andronic III, Abu l-Hasan | cent-ans-mech |
| R7 | Relief fin à l'Est (après la vague 1, selon le disque) | cent-ans-dev |

Intégration : `../gp-omr` (`feat/omr`), tests complets, puis `merge --ff-only` dans main.

## État
- Vague 1 (R1-R6) lancée.
- R6 fini (feat/omr-r6, 25504e100) : campaign_orthodox et campaign_islamic (5 pistes chacun, 24 Mo),
  portraits Andronic III / Abu l-Hasan refaits (0,18 $, cumul OM 8,25 $). Pas de steppe/nordique
  (aucune source libre). Écoute humaine à faire.
