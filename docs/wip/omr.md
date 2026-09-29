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
- Vague 1 (R1-R6) lancée. R7 lancé après R5 (96 Go libres).
- R6 fini (feat/omr-r6, 25504e100) : campaign_orthodox et campaign_islamic (5 pistes chacun, 24 Mo),
  portraits Andronic III / Abu l-Hasan refaits (0,18 $, cumul OM 8,25 $). Pas de steppe/nordique
  (aucune source libre). Écoute humaine à faire.
- R2 fini (feat/omr-r2, 02c77b1ba) : chargement ≈ ×2 plus rapide (≈ 6 s estimé au calme), relief BC5
  + zones humides BC1 (ADR 0118, numéro à vérifier : 0117 pris par tw2-t4), +131 Mo au dépôt.
  RSS ≈ 2,3 Go (cible 2,2 non démontrée, machine chargée). da7d (declutter < 4 ms) à relancer au calme.
  Export embarque encore les PNG du relief (134 Mo) inutiles.
- Disque : 13 Go libres ; profils cargo OM (om1-3, i1, i1main, ≈ 10,8 Go) supprimés, espace retenu
  par les instantanés locaux Time Machine. R7 en attente d'espace.
- R4 fini (feat/omr-r4, 1cc69e53b) : 30 objectifs relus (4 remplacés, 4 reformulés), Eşrefoğulları
  supprimés (éteints 1326, Beyşehir → Hamid, Hızır Bey ajouté sans portrait), Anchialos/Philippopolis
  corrigées, Mazovie, anachronismes de noms, Volok/Gorokhovets/Kamianiets rattachées. Géo partielle
  (provinces, settlements, navgrid ; 8a1fbb9a5). Test cv3_ai_stances recalé (graine 2).
  À l'intégration : portrait Hızır Bey (enveloppe OM) ; conflits possibles avec R3 (données factions D).
  Incertitudes restantes : docs/wip/omr-r4.md.
- R5 fini (feat/omr-r5, fb9e75774) : 10 unités de l'Est (mamelouks, archers des steppes, akıncı, yaya,
  cavalerie serbe, pronoïaires, droujina, cavalerie lituanienne, frères teutoniques, almogavres),
  doctrines IA, unit_looks.json (livrées/robes), emblèmes locaux, codex. Pas de règle core. Conflit
  probable avec R3 sur data/ai/doctrines.json. Brabançons 12 % / archers écossais 84 % (hors bande, préexistant).
- Intégration ../gp-omr (feat/omr) : R6, R2, R4, R5 fusionnés sans conflit. ADR 0118 conservé (0117 réservé par tw2-t4). Attente R1, R3, R7 ; tests complets après.
- R1 fini (feat/omr-r1, ced09f8a2) : décisions IA identiques (turn_digest), CPU de planification ×0,42 (≈ 0,40 s/tour estimé ; à confirmer au calme par turn_perf 10 1 1), ADR 0119. Fusionné dans feat/omr. ADR : R3 a pris 0117, en collision avec tw2-t4 → renuméroter R3 à l'intégration (0120+).
- R7 fini (feat/omr-r7, 5027c6563) : relief fin E1-E2 GLO-90 sur tout le monde OM, pyramide passée au cadre monde (ADR 0121), paquet relief v2 (non publié), hydro/anchors/towns régénérés. Cache 4,98 Go dans ../gp-omr-r7/data/map/pyramid : à échanger avec celui du principal à l'atterrissage ; supprimer tools/geo/raw/pyramid_work/{e0,base,coast}.npy et raw/hydro/cache/links_naturalearth.npz du principal (périmés).
- R3 fini (feat/omr-r3, 901dc0a99) : commise T1 voulue (ADR 0114), garde du seigneur (ADR 0117, seul détenteur du numéro), IA rase/congédie en dette, incite_unrest 12. 464 t. × 10 : banqueroutes 3,98 → 0,69, révoltes 16,0 → 8,1 ; Bourgogne éliminée avant 1400 sur 1/10. Fusionné (conflit cv3_ai_stances : graine 1, à vérifier). Intégration : pytest 1266 ok ; cargo en cours.
