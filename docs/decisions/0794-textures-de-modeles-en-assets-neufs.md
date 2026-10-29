# 0794 — Mémoire de campagne : textures de modèles BC1 dans des assets neufs, mesure avec relief

Date : 2026-10-29. Statut : acceptée. Amende les ADR 0748 et 0776 (amendement du 28/10). Lot MEM4 (boucle juge 2,
itération 1). Détails : `docs/wip/lot-mem.md`, `docs/performance-memoire.md`.

## Contexte
Le juge technique (iter-01, constat ÉLEVÉ) mesurait 6,9 à 12,5 Go d'empreinte de campagne contre un budget de 4 Go.
Trois causes distinctes, établies par mesure (campagne au repos, release, Ultra, hors écran, `footprint`) :

1. **La mesure de l'ADR 0776 (2,8 Go) avait été prise sans le relief.** Un worktree n'a pas `data/map/pyramid` (lien
   vers `~/dev/cent-ans-raw/compose/pyramid`) : la campagne démarrait sans terrain (« No such file » dans le journal).
   Sans relief, la campagne fait bien 2,76 Go ; avec, 5,4 à 6,2 Go. Le budget de 4 Go n'a donc jamais été tenu avec
   le jeu complet ; ce n'est pas une régression de code, c'est un budget calibré sur une mesure incomplète.
2. **L'amendement du 28/10 de l'ADR 0776** (RGBA8 2048 envoyé tel quel au GPU, puis échange en place vers BC1)
   gardait en vie 1,3 Go de textures RGBA8 : Metal alloué 2,96 Go contre 2,13 Go sans elles. L'échange en place de
   l'image n'atteint jamais les bind groups des matériaux (ni le marqueur 4x4, resté affiché, ni la texture BC1 ; le
   `Assets::get_mut` des matériaux ne suffisait pas) et la texture RGBA8 d'origine reste liée. Ces textures
   représentent aussi 4,5 Go d'`IOAccelerator` transitoire pendant le chargement.
3. **Un reste de 2,5 à 3,3 Go de mémoire graphique possédée mais non comptée par Metal** (`Owned physical footprint
   (unmapped) (graphics)` de `footprint`, constant en campagne : 0,46 Go au menu, 2,4 Go en campagne, 2,7 Go en fin de
   banc). Sans lien avec les images (`RenderAssets<GpuImage>` : 1,6 Go, `currentAllocatedSize` : 2,1 Go), les
   pipelines (≈ 100), le relief, les ombres de végétation, la qualité, la limite d'images en vol. Non élucidé.

Les « 144 images non documentées » (497 contre 353) sont de petites images d'interface : héraldique (165 + 13),
`models/dn` (72, déjà en BC1, 0,7 Mo chacune), médaillons, icônes d'encre, enluminures (≈ 20 Mo en tout). Les octets
GPU « non expliqués » ne sont pas dans les images : voir le point 3.

## Décision
1. **Capture au `Added` puis nouvel asset.** `fx/model_mips.rs` prend les pixels de l'albédo opaque des modèles
   `models/dn/` et `models/props_ga/` à l'événement `Added` de l'image (avant le premier envoi au GPU), laisse un
   marqueur 4x4 de la **couleur moyenne** (pas un gris), encode la chaîne BC1 sur un fil de calcul, puis **ajoute une
   image neuve** (`RENDER_WORLD` seul) et **re-pointe les emplacements de texture des matériaux** vers elle
   (`repoint_materials`). Un matériau créé après l'échange prend directement l'image finale. Aucune texture RGBA8 2048
   n'est plus créée sur le GPU. Effet mesuré (campagne au repos, relief chargé) : empreinte stable 6 200 -> 5 400 Mo,
   Metal alloué 2,96 -> 2,13 Go. Le pic de chargement (6,6 Go à t+2 s) est dominé par le décodage des feuilles de sol
   et reste inchangé.
2. **Outils de mesure** (cherry-pick du lot MEM3 resté en branche) : `CENT_ANS_MEMLOG=<s>` (journal d'empreinte, Metal
   alloué, images, maillages), `memory.metal_allocated_mib` dans le dump, `gpucensus` (images réellement présentes côté
   rendu par format et classe de taille, nombre de pipelines), `tools/bevy_bench/mem_protocol.sh`. La limite d'images en
   vol des runs masqués (`gpu_pacing`, 2 par défaut, `CENT_ANS_GPU_LATENCY=0` pour la lever) est **conservée** : en
   bataille elle ramène l'empreinte du banc de 6 540 Mo à 3 560 Mo à temps d'image égaux (le CPU prenait de l'avance
   sur le GPU, une fenêtre réelle est cadencée par la chaîne d'échange) ; en campagne, aucune différence.
3. **Budget** (`perf_thresholds.json`) : l'empreinte de campagne mesurée avec le jeu complet est de 5,4 Go au repos et
   7,3 Go en fin de banc (descentes), pic RSS 4,9 Go. Seuils de **non-régression** : carte 7 800 Mo (pic RSS 5 400), bataille 4 500 Mo (3 560 mesurés, objectif 3 500), mêlée p50 17,5 ms (16,8 mesurés, objectif 16,1) ;
   l'objectif de 4 Go reste noté dans le fichier et dans `docs/performance-memoire.md` tant que le point 3 n'est pas élucidé.
   Chaque mesure doit se faire dans un worktree où `data/map/pyramid` existe.
4. `perf_guard.sh` : `status` est une variable en lecture seule de zsh ; renommée `rc`. La garde tourne de nouveau.

## Conséquences
- Les modèles de lieux restent colorés pendant l'encodage (couleur moyenne), puis passent en BC1 sans à-coup.
- Un lecteur CPU futur de ces textures ne trouve rien dans `Assets<Image>::get(old).data` (déjà vrai en 0776).
- Le budget de 4 Go n'est plus affiché comme tenu. Pistes : élucider les 2,5 Go de mémoire graphique possédée (essai en
  fenêtre réelle impossible pour un agent), pré-cuire les feuilles de sol en KTX2 pour le pic de chargement.
