# Mémoire du client Bevy : budget et mesures

Lot MEM (ADR 0748). Mesures sur Apple M4 Pro, build release, préréglage Ultra, textures hautes, fenêtre masquée
(`client/run_bg.sh --offscreen`, `CENT_ANS_MUTE=1`).

## Comment mesurer
- `--dump-state f.json` : section `memory` = `rss_mib`, `peak_rss_mib`, images (nombre, taille GPU estimée, copies CPU
  conservées, 10 plus grosses côté GPU et côté CPU, avec dimensions / format / mips / `RenderAssetUsages`), maillages
  (copies CPU encore présentes).
- Rapports des bancs (`bench.sh`, `perf_guard.sh`) : bloc `memory.{rss_mib,peak_rss_mib}` (`getrusage`).
- Hors client : `vmmap -summary <pid>` et `footprint <pid>`. **Le RSS (`ps`) sous-estime et fluctue sur macOS** : le
  compresseur mémoire déplace des pages en « swapped » (RSS divisé par deux sans que l'empreinte physique baisse). La
  mesure fiable est `footprint` (empreinte physique, GPU unifié inclus).

## Mesures de base (campagne idle, ~25 s après le lancement)
| Poste | Mo |
|---|---|
| Empreinte physique totale (`footprint`) | 3 300 |
| Graphiques possédés (textures, cibles, tampons GPU, mémoire unifiée) | 1 360 |
| `MALLOC_LARGE` (gros blocs CPU) dont ~730 libérés mais résidents (« empty ») | 1 300 |
| `MALLOC_SMALL` | 520-1 000 |
| RSS `ps` : pic au chargement / stabilisé | 2 450-2 700 / 1 150-2 200 |
| Images : estimation GPU / copies CPU (353 images) | 640 / 261 |
| Maillages : copies CPU (402 sur 409) | 92 |
| Bataille (Crécy) : RSS pic / stabilisé | 2 200 / 1 600 |

Les 5,6 à 7 Go du juge (C1) n'ont pas été reproduits : le pic RSS mesuré est de 2,7 Go (campagne, banc complet avec
descentes) et 2,2 Go (bataille). Ils correspondent plus probablement à une machine chargée ou à une autre mesure (somme
de processus, empreinte avec compresseur). La machine était à une charge de 80-110 pendant le lot : les chiffres
stabilisés varient de ±500 Mo d'une exécution à l'autre.

## Plus gros postes (campagne)
1. Fond de carte composé 14160x12208 BC1 + 14 mips : 110 Mo GPU.
2. Rasters 7080x6104 R8 (41 Mo) et 2 x 3540x3052 RGBA8 (82 Mo), sans mips.
3. Tableaux de sol `tx_campaign_bg` (45 couches 1024 BC1 + normales Rg8) et parcelles (43) : 4 x ~30 Mo.
4. Copies CPU d'images non nécessaires : `castle_wall_varriation_diff.jpg`, `building_albedo/normal_array.jpg`
   (21 Mo chacune), `flame_flipbook.png` (16 Mo), ~40 albédos `battle_ga3` (6 Mo chacun), soit ~100 Mo évitables.
5. Trois images de remplissage de provinces 1770x1526 RGBA8 (10 Mo CPU chacune, nécessaires : écrites à l'exécution).
6. Copies CPU des maillages (92 Mo).
7. Décodage transitoire des feuilles JPEG de sol (4096x11264 = 185 Mo RGBA8 par feuille) : source des blocs
   « empty » du tas.
8-10. Décor de bataille (`castle_wall`, flipbooks) chargé dès la campagne ; tas des simulations ; cibles d'ombre.

## Budget (garde dans `tools/bevy_bench/perf_guard.sh`)
Pic de RSS : **campagne 3 500 Mo, bataille 3 000 Mo** (`perf_thresholds.json`, clé `memory`). Le garde échoue au-delà et
n'est valable que sur machine calme (même règle que les temps d'image).

## Essais écartés
- `malloc_zone_pressure_relief` en tâche de fond : RSS divisé par 2 mais empreinte physique inchangée (pages
  compressées, pas rendues). Abandonné, trompeur.
- Cache disque des tableaux de sol construits (évite le décodage des feuilles) : pic -5 % seulement, empreinte stabilisée
  inchangée (blocs « large » toujours 1,27 Go). Abandonné.

## Pistes non faites (mesure fiable requise, machine calme)
- Passer en `RENDER_WORLD` seul les images listées au point 4 (~100 Mo), après vérification qu'aucun lecteur CPU ne les
  relit.
- Découper le décodage des feuilles de sol en bandes ou pré-cuire en KTX2 (ADR 0724 : consommateurs CPU à adapter).
- Décharger les ressources de campagne à l'entrée en bataille (mesurer `footprint` avant/après).

## Amendement ADR 0776 (lot MEM2, 23/10) : régression de campagne
Les « mesures de base » ci-dessus avaient été prises sans les modèles de lieux `models/dn/*` (assets non suivis absents
du worktree de mesure). Avec eux, la campagne montait à 6,5-6,8 Go d'empreinte (juge : 10-12 Go sur machine chargée).
Cause : 66 textures 2048x2048 RGBA8 (21 Mo GPU + 21 Mo CPU chacune) + ~1 Go de tampons de décodage retenus.
**Attention de mesure** : copier `game/assets` (non suivi) et `out/ktx2_cache` depuis le checkout principal dans un
worktree avant de mesurer (`rsync -a --ignore-existing`).

| Campagne, release, Ultra, masqué | Empreinte (Mo) | Pic RSS (Mo) |
|---|---|---|
| Avant (c26909646) | 6 550-6 760 | ~4 900 |
| + BC1 / GPU seul des textures dn | 4 200-4 850 (variance libmalloc) | ~5 000 |
| + capture avant premier envoi GPU | 2 400-3 800 (variance libmalloc) | 3 100 |
| + mimalloc (état final) | 2 750-2 820 | 3 600 |

Images : copies CPU 1 537 -> ~180 Mo, estimation GPU 1 916 -> ~690 Mo. Maillages : copies CPU 118 Mo (inchangé).
fps (A/B alternés, système vs mimalloc, charge 10-25) : carte pan d30 p50 7,6-8,2 vs 7,8-9,0 ms (référence juge
9,8-10,2) ; bataille calm 73-76 vs 75-77 fps, mêlée 52-59 vs 56-59 fps (bruit de charge, pas de régression).
Mesure : `memory.footprint_mib` (dump et rapports de banc, `proc_pid_rusage`), `footprint <pid>`, `vmmap -summary`.

### Points ouverts
- Bataille : l'empreinte oscille de 3,8 Go à 5,8-6,3 Go selon `IOAccelerator (graphics)` (0,4 <-> 2,3 Go, par
  tranches de ~1 min), avec ou sans mimalloc, avec ou sans volumétrique (`CENT_ANS_VOLUMETRIC=off` : pics plus rares).
  Hors tas et hors textures ; cause non identifiée (ressources Metal en vol sous vsync coupé / fenêtre masquée ?).
  Plancher bataille 3,8 Go > budget 3,5 Go ; le garde bataille est donc large (7 000) en attendant.
- Décharge des ressources de campagne à l'entrée en bataille : non mesurée.
- Feuilles de sol de campagne (JPEG 185 Mo RGBA8 décodés, transitoires) : pré-cuisson KTX2/BC7 non faite (le pipeline
  `bake-ktx2` exclut la campagne) ; mimalloc rend les tampons à l'OS, l'enjeu restant est le temps de chargement.

## Amendement ADR 0796 (lot MEM4, 29/10) : mesure avec relief, textures de modèles en assets neufs
- **Piège de mesure n° 2** : un worktree sans `data/map/pyramid` démarre une campagne sans terrain (journal : « No such
  file or directory » sur `relief_pyramid.json`). Les 2,8 Go de l'ADR 0776 étaient mesurés ainsi. Lier
  `ln -s ~/dev/cent-ans-raw/compose/pyramid data/map/pyramid` avant toute mesure.
- Campagne au repos (release, Ultra, hors écran, relief chargé), Mo d'empreinte : 6 200 (avant) -> 5 400 (après) ; Metal
  alloué 2,96 -> 2,13 Go ; sans relief 2 760. En fin de banc carte (descentes) : 7 350 ; pic RSS 4 860.
- Postes : `RenderAssets<GpuImage>` 1,6 Go, Metal alloué 2,1 Go, empreinte 5,4 Go : il reste ~3 Go de graphiques
  possédés (`Owned physical footprint (unmapped) (graphics)` 2,4-2,7 Go) que ni les images, ni les pipelines (≈ 100), ni
  la végétation, ni le terrain, ni la limite d'images en vol n'expliquent. Menu : 632 Mo (Metal 171).
- Outils : `CENT_ANS_MEMLOG=<s>` (journal `memlog` + `gpucensus`), `tools/bevy_bench/mem_protocol.sh idle|bench|battle`,
  `memlog_view.py`, `vmmap_regions.py`, `memory.metal_allocated_mib` du dump.
- Mesure fps (bataille, `bench.sh` en build `profile`) : la mêlée est liée au GPU (le thread de rendu attend le GPU 6 ms par
  image) ; volumétrique ~2 ms, ombres des figurines ~1,5 ms (A/B, machine chargée : lire en relatif). Carte : les pics
  de 350 à 1 100 ms viennent de `PipelineCache::process_pipeline_queue_system` (compilation synchrone des pipelines sous
  macOS, Bevy ne la rend pas asynchrone) au premier passage de chaque variante ; piste : préchauffer les pipelines
  pendant le voile de chargement.
- Bataille (banc Crécy, release, masqué) : 6 540 Mo sans limite d'images en vol, 3 560 Mo avec (`gpu_pacing`, 2 par
  défaut, mêmes temps d'image) : l'oscillation `IOAccelerator (graphics)` du point ouvert de l'ADR 0776 était le CPU
  qui prenait de l'avance sur le GPU en fenêtre masquée, pas une fuite. Garde bataille ramenée à 4 500 Mo.
