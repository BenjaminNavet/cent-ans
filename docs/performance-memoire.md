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
