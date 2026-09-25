# Batailles épiques (session du 25/09)

Demande du joueur : rendre les batailles épiques. Carte de bataille réaliste et accidentée (collines,
rivières, ponts, villages), horizon peint ou lointain hors de la zone jouable (mer, montagnes), unités
massives portant chacune un étendard (une figurine porte-étendard), cris et fracas d'armes quand la
caméra s'approche d'une mêlée, et toute autre idée qui sert l'épique. Autonomie complète, **plafond de
20 $** pour les assets générés (consigné dans `docs/budget.md`, section « Batailles épiques »).

Réponses du joueur au lancement :
- échelle : **15 000 soldats et plus** au total ;
- générateur procédural enrichi **et** 3 cartes historiques faites à la main : **Crécy, Poitiers, Azincourt** ;
- horizon **hybride** : relief réel autour du site en maillage lointain + panorama peint (IA).

Worktree d'intégration : `../gp-epic-merge` (branche `integration/epic`), fusion ff-only dans main,
commits avec chemins explicites. Au plus 6 agents à la fois (machine partagée avec l'orchestrateur de
nuit : NV2, SG3, EQ1, PF1, DP2, AR1).

## Constat de départ (audit du 25/09)
- Champ 1200 × 800 m, grille de relief 10 m ; anneaux lointains décoratifs déjà présents (3 km, ±7 km à
  200 m) mais sans panorama ni relief réel ; caméra jusqu'à 900 m, plan lointain à 16 km.
- **Plafond `MAX_ON_FIELD = 20` régiments par camp** (`sim/reinforcements.rs:13`) : ≈ 4 800 soldats au
  plus sur le terrain. Banc T2 : 28 752 soldats à 30 i/s ; BV3 : 11 800 à 55 i/s avec imposteurs.
- Une seule rivière (18 m, 2 gués, eau profonde lente mais franchissable), **aucun pont**, routes
  purement décoratives, un seul village (6 à 11 bâtiments) ou une ferme.
- Étendard porté par une figurine ordinaire du rang (`bearer_rank`), pas de modèle dédié, visible à
  moins de 220 m ; pas de chute ni de prise d'étendard.
- Audio : une seule « nappe » de mêlée par type, au barycentre ; ≈ 60 clips de bataille (4 chocs
  d'épée, 7 râles, 3 cris de charge) : trop peu pour une foule.
- Aucune carte de bataille fixe ; « Crécy » n'existe que dans des illustrations.

## Lots

| Lot | Objet | ADR | Dépend de | État |
|---|---|---|---|---|
| EP1 | Échelle massive : champ plus grand selon l'effectif, plafond de régiments relevé, rendu 15 000+ (imposteurs très lointains, budget d'animation par distance), banc ≥ 40 i/s | 0031 | — | **fusionné** 13506bf1 |
| EP2 | Horizon : relief réel (DEM) autour du lieu en anneau lointain, panoramas peints par région (mer, Alpes, Pyrénées, collines), silhouettes lointaines (clocher, château), brume de chaleur/fumée de camp | 0032 | — | **fusionné** 13506bf1 |
| EP3 | Eau et chemins : plusieurs cours d'eau et ruisseaux, ponts de bois et de pierre (goulots), gués multiples, routes qui accélèrent la marche, IA qui tient ponts et gués | 0033 | — | **fusionné** 13506bf1 |
| EP4 | Son de mêlée de proximité : émetteurs par front de mêlée, couches proche/moyen/lointain, grande banque CC0 (chocs, cris, râles, chevaux, ordres), foule qui monte avec l'effectif | — | — | **fusionné** a30b461b (27 clips CC0, 112 générés ; ordres criés sans source CC0 ; volumes à régler à l'oreille) |
| EP5 | Étendards : figurine porte-étendard dédiée (pose et clips), musiciens (tambours, trompettes), étendard qui tombe, relevé ou pris (moral, écran de fin) | 0034 | — | **fusionné** 13506bf1 |
| EP6 | Villages et décor du champ : hameaux variés, moulin à vent/à eau, église et cimetière, manoir fortifié, vignes, vergers, meules, charrettes, camp et convoi derrière les lignes, pieux | — | EP3 | en cours (repris 25/09 soir) |
| EP7 | Cartes historiques Crécy (26/08/1346), Poitiers (19/09/1356), Azincourt (25/10/1415) : relief réel, décor d'époque, déploiement historique, entrée depuis la campagne et le menu | 0035 | EP1-EP3, EP6 | vague 2 |
| EP8 | Mise en scène : heure du jour (aube, crépuscule), ombres de nuages, poussière des charges, fumées, oiseaux qui s'envolent, caméra cinématique au premier choc | 0055 | EP2 | **fusionné** 3805ef66 |

## Budget (plafond 20 $)
Prévu : panoramas EP2 (≈ 12 images, ≈ 0,6 $), fonds de cartes historiques EP7 (≈ 0,3 $). Sons : banques
CC0 gratuites (Freesound, `tools/cent_ans_tools/freesound_search.py`). Réserve pour génération 3D
éventuelle (porte-étendard, moulin) si le kit Blender ne suffit pas.

## Reprise
1. `git worktree list` : worktrees des lots EP*, fichier wip `docs/wip/ep<N>-*.md` dans chacun.
2. Fusion : dans `../gp-epic-merge`, `git merge main` puis la branche du lot ; tests ; ff-only dans main.

## Décisions
- 25/09 : ADR 0031 à 0035 réservées pour ce chantier (0029-0030 laissées à la nuit : PF1 prévoit un ADR).
- Taille du champ : EP1 rend les dimensions du champ paramétriques (plus de constantes figées) ; EP3 et
  EP6 ne doivent jamais supposer 1200 × 800 et lisent les dimensions du champ.

## Intégration (25/09, après la coupure de quota)
- EP1, EP5 puis EP2 fusionnés dans `integration/epic` : conflits `sim.rs` (champs de `BattleSim`,
  les deux gardés) et `battle_scene.gd` (brume au sol paramétrique EP1 + atmosphère EP2).
  clippy, cargo test, pytest (436), `ep2_horizon_test`, `ep4_audio_test` : OK.
- **Bloquant avant ff-only dans main** : le smoke général plante sur la carte de campagne
  (`Container::_sort_children. Message queue out of memory`, soupçon UI1 + panneau EQ1), bug de main
  pris en charge par l'orchestrateur de nuit (agent SM1). Attendre son correctif, `git merge main`,
  smoke vert, puis ff-only.
- EP3 relancé dans son worktree (`ep3-eau-chemins`) : règles de simulation, ADR 0033.
- EP3 fusionné dans `integration/epic` (a1465985) : conflits `field.rs`, `sim.rs`, `site.rs`, `ai.rs`,
  `battle_request.rs` (EP5 + DF1 venu de main par EP3), `real_data.rs`, `battle_terrain.gd` ; tout
  gardé. `hydro::battle_lines` supprimé au profit de `field.attacker_line_z()/defender_line_z()`,
  `shape_river` reçoit la largeur du champ, `generate_sized` pose aussi ponts et routes. Test
  `automatic_deployment_stays_in_the_zone_on_rivers` (paliers skirmish/large/epic, rivière) : le
  déploiement automatique reste dans la zone et hors de l'eau profonde. Le message « doivent être
  placés dans votre zone » vu par EP3 vient de `--deploy-shot`, qui place exprès un régiment hors
  zone pour montrer le refus. cargo test (658), clippy, pytest (450), `ep2_horizon_test`,
  `ep4_audio_test`, capture de déploiement palier epic avec rivière : OK. Digests inchangés.
- Fusion de main (R4 IA de position, SG3 servants de siège, correctif smoke a2a79044) : conflits
  `ai.rs` (berge EP3 d'abord quand le passage est tenable, sinon `defensive_ground` R4 ; ADR 0033
  § « Cohabitation avec R4 »), `unit.rs` (`standard` + `seen_at`), scripts Blender (clips et
  figurines des deux lots), puis `human.bones.bin` et manifeste régénérés par Blender
  (`--only crew_0,crew_1,standard_0,standard_1,musician_0,musician_1` : meshes identiques, rig
  humain rebâti avec `std_*`/`drum_*`/`horn_*` et `crank`/`haul`/`load`/`swab`/`push`). Digests
  `b6.rs` recalculés (graines 3 et 11 : vainqueur attaquant inchangé). `ep1_scale` 60 × 60 : le
  défenseur R4 reçoit l'attaque sur son terrain, pic de 15-22 régiments en mêlée (seuil ramené à
  12, compté à chaque pas).
  Vérifié (9c499c32) : fmt, clippy, cargo test (688), `build.sh`, pytest (531), import Godot,
  `ep2_horizon_test`, `ep4_audio_test`, smoke entier vert (28 « smoke OK », code 0), captures
  `--standard-shot=line|foot` (étendards et pennons visibles) et `sg3_siege_shot` (servants de la
  bombarde et du trébuchet animés).
- **25/09 : vague 1 dans main (ff-only, 13506bf1)** : EP1, EP2, EP3, EP5 (EP4 déjà fusionné).
  Dernières fusions de main : `cli.py` (sous-commandes EP2 `horizon-panoramas` + AR1 `art-plates`,
  `geo horizon` + `geo pyramid`). Vérifié : cargo test (692), pytest (545), smoke 28 OK code 0.

## Points ouverts après la vague 1
- Équilibrage : avec R4, le défenseur gagne à forces égales à l'échelle épique (graines 3, 5, 11) ;
  sonde `ep3_probe` (rivière) non relancée.
- `--standard-shot=foot` cadre parfois dans une pile de pont sur un site EP3 : décaler la caméra.
- `battle_skinned_poses.py` : défauts ruff (docstrings) antérieurs, venus d'EP5.
- Au-delà de 172 régiments présents, le coût est dans le script par régiment (EP1).

## Prochaine étape
Vague 2 : EP6 (villages et décor du champ), puis EP7 (Crécy, Poitiers, Azincourt) et EP8 (mise en
scène). EP6 et EP8 peuvent partir en parallèle ; EP7 après EP6.

### 25/09 soir
- EP8 fusionné dans main (3805ef66) : heure du jour (`BattleSim::range_factor`, midi par défaut, donc
  sans effet sur l'équilibre), fumées, poussière, caméra ; ADR 0055.
- EP6 (villages) et EP9 (batailles décisives) coupés par le quota à 15 h 50, relancés depuis leur
  `docs/wip/ep6-villages-decor.md` et `docs/wip/ep9-batailles-decisives.md`.
- EP7 attend EP6 (API de placement explicite) et utilise `set_start_hour` d'EP8.
