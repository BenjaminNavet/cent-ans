# PF1 — performances (préréglages, zoom comté, fuite particules, user://)

Agent PF1, 25/09. Machine partagée : **charge moyenne 20 à 230** pendant les mesures (autres
agents : builds Rust, Godot). Les comparaisons fiables sont celles faites dans le même processus
(`--map-ab`, niveaux alternés sur 4 tours) et les compteurs (primitives, appels de dessin).
Chaque mesure est faite 2 fois ; on donne la médiane des deux (ou l'intervalle).

## Tâches
1. [x] Préréglages Basse / Moyenne qui réduisent vraiment la géométrie et les effets
2. [x] Zoom comté en Haute tenu à 16,7 ms (relief fin en blocs + LOD, filtre d'ombre de la carte)
3. [x] Fuite « ParticlesShaderRD were never freed » : corrigée, vérifiée sur le jeu exporté
4. [x] user:// du jeu exporté isolé + copie unique des fichiers du joueur (ADR 0031), vérifié sur l'export

## Ce que règle chaque niveau (`RenderQuality.PRESETS`)
| | Basse | Moyenne | Haute | Ultra |
|---|---|---|---|---|
| MSAA | coupé | ×2 | ×2 | ×4 |
| SSAO / SSIL | non / non | moyen / non | haut / oui | ultra / oui (+ SDFGI en bataille) |
| Halo, brume volumétrique | non, non | oui, non | oui, par mauvais temps | oui, toujours |
| Atlas d'ombre, filtre (bataille) | 2048, doux bas | 4096, moyen | 8192, haut | 8192, ultra |
| Ombres de la carte : portée, cascades, filtre | ×0,6, 2, bas | ×0,8, 4, bas | ×1, 4, **moyen** | ×1,2, 4, haut |
| Relief fin (zoom comté) | **aucun** | oui, LOD ×2 plus tôt | oui | oui, LOD ×2 plus tard |
| LOD proche du relief de la carte | ×0,6 | ×0,8 | ×1 | ×1,2 |
| Arbres de la carte : part, portée du détail, ombres jusqu'à d= | 50 %, ×0,6, jamais | 75 %, ×0,8, 200 | 100 %, ×1, 300 | 100 %, ×1,3, 400 |
| Figurines : LOD, ombres (imposteurs ≥ 80 %) | ×0,55 | ×0,75 | ×1 | ×1,3 |
| Arbres et haies de bataille (distances de LOD) | ×0,55 | ×0,75 | ×1 | ×1,3 |
| Herbe de bataille (rayon ; touffes ∝ rayon²) | ×0,55 | ×0,75 | ×1 | ×1,2 |
| Particules (pluie, neige, sang, fumée, poussière, éclats) | 35 % | 60 % | 100 % | 100 % |

Pas de réflexions en espace écran dans le jeu (SSR jamais activé) ; l'eau de bataille lit la
texture d'écran (réfraction), inchangée.

## Mesures — carte (`--map-ab=<d>`, Vulkan, GPU ms médian, 2 exécutions)
| d | Haute | Moyenne | Basse | Ultra | primitives H / M / B / U | dessins H / B |
|---|---|---|---|---|---|---|
| 1500 (parchemin) | 5,7-5,9 | 6,1-6,7 | 6,8-7,0 | 5,9 | 378 k partout | 1 772 |
| 1250 (fondu) | 9,5-10,3 | 8,8-9,2 | **5,4** | 11,2 | 433 k partout | 796 |
| 491 (défaut) | 18,9-19,1 | 11,9-12,0 | **6,9** | 26,5-26,9 | 2,29 M / 1,89 M / 1,34 M / 2,40 M | 1 020 / 808 |
| 150 (comté) | 20,0-21,9 | 11,9-12,7 | **6,4** | 32,3-34,9 | 6,02 M / 4,50 M / 1,66 M / 9,97 M | 1 410 / 950 |
Basse / Haute au zoom comté : **3,1 à 3,4 fois plus rapide** (GPU), primitives ÷ 3,6.
Au parchemin (1500), le coût est la couche 2D (mêmes primitives) : les niveaux n'y changent rien.

## Mesures — bataille (banc T8, `--units=n --bench-at=40 --quality=<niveau>`, Vulkan, 2 exécutions)
| régiments par camp | niveau | GPU ms | primitives | dessins | image médiane (CPU chargé) |
|---|---|---|---|---|---|
| 20 (4 780 soldats) | Haute | 9,8-9,9 | 2,18 M | 740 | 10,0 |
| | Moyenne | 6,2 | 1,25 M | 705 | 7,6 |
| | Basse | **3,2** | 0,82 M | 692 | 6,6-6,7 |
| | Ultra | 16,6 | 2,68 M | 756 | 16,7-16,8 |
| 50 (11 980 soldats) | Haute | 8,6 | 1,21 M | 670 | 10,5-10,8 |
| | Moyenne | 5,1 | 0,88 M | 657 | 10,0-10,4 |
| | Basse | **2,8** | 0,65 M | 644 | 9,3 |
| | Ultra | 14,7-14,9 | 1,47 M | 688 | 15,3 |
Basse / Haute : GPU ÷ 3,1 ; avec 50 régiments l'image est limitée par le CPU (simulation debug,
machine chargée), pas par le rendu.

## Zoom comté en Haute (tâche 2)
Diagnostic (`--map-ab=150`, coûts isolés) : ombres ≈ 11 ms GPU dont ≈ 5 ms pour le **filtre
doux PCSS** (soleil de la carte à 0,8° de diamètre angulaire) ; relief fin ≈ 5 ms dont l'essentiel
n'est **pas** le nombre de triangles mais leur taille : 1 à 3 pixels loin du centre, chaque
pixel du shader de terrain (lourd) est ombré plusieurs fois (quads 2 × 2, MSAA).
- `FineTerrainJob` : tuile fine découpée en 16 blocs de 64 quads (écartés hors champ et hors
  cascade d'ombre), chacun avec 3 LOD Godot (pas ×2, ×4, ×8). Clé de LOD = min(erreur de hauteur
  réelle du niveau, arête du niveau plus fin / 6 px) : on passe au niveau grossier dès que
  l'erreur fait < 1 px ou que les triangles fins font < 6 px. Les normales viennent de la
  heightmap dans le shader : l'ombrage ne change pas. Captures comparées (relief fin complet,
  LOD, sans relief fin) : identiques à l'œil.
- Filtre des ombres douces de la carte : moyen en Haute (au lieu de haut), le filtre de bataille
  reste haut (`map_soft_shadows`, contexte suivi par `RenderQuality.active_context`). Captures :
  pas de différence visible.
- MSAA ×2 gardé.
Résultat (Metal, jeu lancé par l'éditeur, 1440×900 Retina) :
| | avant | après |
|---|---|---|
| d=150 plafonné (médiane / p95) | 18,6 / 29,9 ms | **16,68 / 16,9-17,1 ms** (2 exécutions) |
| d=150 non plafonné (`--uncapped`) | 20,8-21,6 ms | 16,9-17,8 ms (charge 19-28) |
| primitives d=150 | 7,25 M | 6,02 M |
| d=491 non plafonné | 13,6-15,7 ms | 13,2 ms |

## Outils
- `--map-ab` : attend relief fin et végétation, revise le point avant chaque configuration
  (un travelling d'arrivée faussait la distance), rend primitives et dessins, `--ab-shots=<dossier>`
  (une capture par configuration). Configurations ajoutées : `no_veg`, `veg_noshadow`,
  `veg_density:x`, `terrain_noshadow`, `fine_noshadow`, `fine_hidden`, `fine_step:n`,
  `fine_bias:x`, `splits2`, `shadow_range:x`, `soft_medium`, `soft_low`, `soft_hard`.
  (Correction : `no_shadows` coupait les ombres des configurations suivantes.)
- Banc de bataille : `primitives` et `draw_calls` dans `BENCH_JSON`.
- Test : `godot --headless --path game --script res://tests/pf1_quality_test.gd`.

## Fuite de particules (tâche 3)
Bissection sur le jeu exporté (`-- --journey --no-battle` + options de test temporaires) :
fuite présente dès la carte seule, absente avec `--no-map-weather` ; due à la **pluie** de CM2
(il pleut sur Paris au tour 1) ; la neige forcée (`--map-weather=snow`) fuit aussi. Ni la
turbulence, ni le recalage des paramètres au zoom, ni le retrait du matériau dans `_exit_tree` ne
changent rien : une particule **émettrice** à `ParticleProcessMaterial` laissait une entrée du
cache statique de shaders du moteur à la fermeture (modèle d'export seulement ; pas dans l'éditeur).
Correction : pluie et neige de campagne en `ShaderMaterial` de particules maison
(`game/shaders/campaign_precipitation.gdshader` : boîte d'émission, cône, vitesses, ondulation de
la neige ; mêmes paramètres pilotés par le zoom). Jeu exporté : parcours complet (carte, fins de
tour, sauvegarde, bataille) et carte sous neige forcée, **journal sans erreur à la fermeture**.
Captures pluie avant/après : identiques.

## Dossier utilisateur (tâche 4, ADR 0031)
Jeu exporté : `~/Library/Application Support/Cent Ans` (`use_custom_user_dir.template`), éditeur
et tests inchangés. Premier lancement de l'export : « 14 fichiers repris » (réglages, codex,
3 sauvegardes automatiques + sauvegarde rapide avec métadonnées et vignettes ; `smoke.json`
écarté), marqueur écrit ; lancements suivants : aucune copie.

## État final
Terminé. `main` fusionné (EP4, UI1, PB1, playlists, UX1/UX2, ZG, MF1 : aucun ne touche
`render_quality.gd` ni les fichiers PF1, fusion sans conflit). fmt, clippy -D warnings, cargo test,
build.sh (debug et release), pytest (405), import, `rl1_quality_test`, `pf1_quality_test` : OK.
Smoke : 3 échecs « music playlist too short » venus de `main` (commit cb1b4416 : les listes de
`data/audio/music.json` ne sont pas lues quand le smoke pointe `MapPaths` sur les fixtures) ;
aucun échec lié à PF1 (27 « smoke OK » avant cette fusion).

## Points ouverts
- Smoke / playlists (voir ci-dessus) : à corriger par le lot playlists (fixture ou repli).
- ADR 0036 (ZG, relief streamé) va réécrire le relief de la carte : le découpage en blocs + LOD
  de `FineTerrainJob` / `TerrainBuilder` (PF1) est à reprendre ou à fusionner avec soin.
- Au parchemin (d=1500), 1 772 appels de dessin pour 378 k primitives, coût 2D indépendant du
  niveau : piste pour Basse (regrouper les marqueurs).
- Les distances de LOD de bataille et l'herbe sont lues à la construction de la bataille : un
  changement de niveau en pleine bataille vaut pour la suivante (effets d'écran et particules :
  immédiats).
- Mesures faites sous une charge de 20 à 230 : les temps absolus bougent d'une exécution à
  l'autre ; les rapports entre niveaux (même processus) sont stables.
