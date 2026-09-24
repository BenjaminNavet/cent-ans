# Audit A5 — technique (rendu, performance, audio, robustesse)

État : terminé (reprise après crash machine, mesures refaites). Auditeur A5, session de nuit 7, 2026-09-24.
Lecture seule sur `game/`, `core/`, `data/`. Machine : M4 Pro partagée avec d'autres sessions
(compilations cargo concurrentes) : toutes les mesures sont bruitées (±30 %).

## 1. Rendu (constats statiques)

- Pipeline : Forward+ (`project.godot`), Godot 4.7.2, MSAA 3D ×2 + FXAA, SSAO qualité 3 (ultra),
  ombres directionnelles `size=8192` (≈ 256 Mo de VRAM pour l'atlas seul), filtre doux qualité 3.
- Campagne (`scenes/campaign_map.tscn`, `scripts/visual/campaign_atmosphere.gd`) : ciel
  `ProceduralSkyMaterial`, AgX, SSAO, brouillard de profondeur adapté au zoom, DOF lointain
  (effet maquette), ombres coupées au-delà de d=650. Pas de glow, pas de SSIL/SDFGI/SSR/brouillard
  volumétrique.
- Bataille (`scripts/battle/battle_atmosphere.gd`) : ciel procédural maison
  (`battle_sky.gdshader`, radiance 256), AgX, SSAO, glow léger, brouillard exponentiel + perspective
  aérienne, 4 préréglages météo (clair, brouillard, pluie, neige), ombres PSSM 4 cascades à 420 m.
  Pas de SSIL ni de brouillard volumétrique ; pluie/neige en particules GPU accrochées à la caméra.
- LOD : `visibility_range` utilisé (village de bataille, arbres de bataille, colonies), LOD
  maison pour terrain (3 niveaux dont relief fin 8192² en tâches de fond), végétation (2 maillages
  + éclaircissement shader), soldats (3 maillages, ombres coupées à 190 m). Aucun
  `OccluderInstance3D`, aucun `lod_bias`, pas d'imposteurs pour les arbres lointains.
- MultiMesh généralisé (soldats par régiment, cadavres, végétation, hameaux, maisons de siège
  regroupées par `battle_siege_batcher.gd`).
- Shaders (`game/shaders`, 2 678 lignes) : `terrain.gdshader` (536 l., splat PBR Poly Haven en
  Texture2DArray, normales dérivées de la heightmap, frontières SDF), `battle_soldier.gdshader`
  (771 l., animation par sommet), `water.gdshader` (campagne, sans normal map ni réfraction),
  `battle_water.gdshader` (normal map, profondeur et écran : réfraction), `foliage` /
  `battle_foliage` (alpha scissor, vent), `battle_grass` (herbe instanciée procédurale).
- Matériaux : 52 `StandardMaterial3D` créés en code (16 fichiers), aucun `ORMMaterial3D`,
  aucune `ReflectionProbe`, aucun `Decal` hors anneau d'armée. `road_line.gdshader` n'a pas de
  `.uid` (fichier non importé ou non versionné).

## 2. Performance

Protocole : fenêtre Metal, `--disable-vsync`, lib Rust **debug** (celle que charge
`godot --path game`, voir 2.3). Le plafond à 60 i/s signalé en V6 n'est pas systématique : il
apparaît selon l'état de la fenêtre (même banc : 59 puis 96 i/s de moyenne). Les i/s ne sont donc
comparables qu'avec les primitives / appels de dessin ; Metal ne remonte pas le temps GPU
(`gpu_ms` = 0), le seul indicateur fiable reste la médiane du temps d'image.

### 2.1 Campagne (`--fps-probe --focus=2213,1924,<d>`, Paris)

| Vue | i/s | primitives | appels de dessin | remarques |
|---|---|---|---|---|
| d=1500 (royaume) | 62 | 0,29 M | 528 | |
| d=491 (défaut) | 61 | 2,10 M | 594 | végétation |
| **d=150 (comté)** | **47-49** | **8,8-9,4 M** | 1 351-1 369 | 4 tuiles de relief fin, 661 hameaux |
| d=150, `--no-fine-terrain` | 60 (plafond) | 3,1 M | 1 369 | |
| d=150, `--fine-step=2` | 60 (plafond) | 4,6 M | 1 371 | |
| d=45 (très proche) | 60 | 9,8 M | 988 | |

**Goulet n° 1 de la carte** : le relief fin C6 (`fine_step=1`, un sommet toutes les 0,5 unité,
jusqu'à 4 tuiles) ajoute ≈ 6 M primitives (passe d'ombre comprise) et fait passer d=150 sous
60 i/s. `fine_step=2` divise ce coût par 4 sans perte visible à cette distance : pas adaptatif
selon le zoom (2 au-delà de d≈80, 1 en deçà) = gain immédiat. Le CPU de rendu reste faible
(0,4-0,9 ms), `terrain_lod_ms` 0,1-0,2 ms, couches C6 0,1-0,4 ms : **la carte est limitée par le
GPU (géométrie + ombres), pas par GDScript.**

### 2.2 Bataille (`battle.tscn -- --benchmark --units=<n> --bench-at=40`)

| Scénario | soldats | i/s moyen | médiane | primitives | appels |
|---|---|---|---|---|---|
| défaut, t=40 s | 1 145 | 59 | 60 (plafond) | 1,38 M | 498 |
| 48 régiments, déploiement | 5 760 | 59 | 60 (plafond) | 1,94 M | 610 |
| 48 régiments, t=40 s | 5 733-5 742 | 59-96 | 60-126 | 2,3-2,5 M | 659-726 |
| 80 régiments, t=40 s | 9 581 | 76 | 103 | 2,28 M | 627 |
| 120 régiments, t=40 s | 14 400 | — | — | — | — |

- Le rendu des soldats tient bien : MultiMesh par régiment, 3 LOD, ombres coupées à 190 m ;
  de 1 145 à 9 581 soldats, appels +26 % et primitives +65 % seulement.
- 120 régiments et `--closeup` à 80 régiments : aucune ligne de résultat après 98-220 s
  (avance rapide de 40 s de simulation en Rust debug trop lente, et/ou processus interrompu par
  une autre session). Une exécution groupée a aussi imprimé « 8080 units, 969 492 soldiers »
  (non reproduit, à surveiller : possible accumulation de `_pad_setup`).
- Deux exécutions sur huit se sont terminées sans résultat avec le code 0 : le banc n'est pas
  robuste (pas de délai de garde, pas de code d'erreur).
- GDScript chaud par image (`battle_scene.gd::_process`) : `get_units()` (tableau de
  dictionnaires, 80 entrées), 8 × `get_soldier_buffer` + `slice` (copies), `get_siege()`
  (dictionnaire complet) chaque image, et `tick(delta*speed)` à pas variable. Acceptable à
  9 600 soldats ; à surveiller au-delà (tampons compactés, siège en différentiel).

### 2.3 Fin de tour, chargement, mémoire

- **Fin de tour, lib debug** (script jetable `CampaignSim.end_turn` × 40, France, graine 1337) :
  moyenne **4,5 s**, médiane 4,1 s, **pic 18,4 s**. En release, A2 mesure ≈ 85 ms / tour
  (17 s pour 200 tours, IA comprise, machine chargée) : **facteur ≈ 50**. Or l'éditeur et tous les
  lancements `godot --path game` chargent `libcent_ans.debug.dylib`, compilée en `opt-level=0`
  (seuls `png`/`flate2`… sont optimisés dans `core/Cargo.toml`). C'est le premier goulet ressenti
  en développement et dans toutes les captures ; la batterie `cargo test` en souffre aussi
  (7 min 9 s, dont `f7_events.rs` 134 s et `campaign_ai.rs` 70 s).
- `libcent_ans.release.dylib` date du 24/09 15:05 alors que la debug date de 23:37 : la release
  n'est reconstruite qu'à l'export (`tools/export_macos.sh`).
- Pont : 20 × (`get_army_ids` + `get_army` de toutes les armées) = 45 ms en debug (≈ 2 ms
  par balayage), négligeable.
- Chargement : `new_campaign` 0,55 s (debug) ; scène de campagne 1,7-2,3 s
  (`heightmap` 0,12 s, masques 0,36 s, terrain 0,56-0,66 s, décor 0,31-0,33 s) ; démarrage à chaud de
  la végétation ≈ 1,2 s (V6). Total < 4 s : correct.
- Mémoire (pic `time -l`) : campagne d=150 **1,19 Go RSS / 2,2 Go d'empreinte** ; bataille
  48 régiments 0,59 Go / 1,34 Go. L'atlas d'ombre 8192² (≈ 256 Mo) pèse dans les deux.
- 35 avertissements « Image format RGBFloat not supported by hardware, converting to
  RGBAFloat » par chargement de carte (conversion CPU + 33 % de mémoire en plus pour ces
  images ; source non localisée dans les scripts, probablement une ressource importée en RGBF).

## 3. Audio (constats)

- Inventaire : 3 musiques (`campaign`, `court`, `war`, 72-80 s chacune, ≈ 2,3 Mo) et 10 effets
  (0,06 à 3,2 s). Total 2,5 Mo. Une seule piste de guerre pour toutes les batailles.
- Bus créés en code (`Musique`, `Effets`, `BatailleMusique` avec passe-bas) ; pas de
  `default_bus_layout.tres`, donc pas de bus Ambiance/Voix, pas de compresseur/limiteur sur Master.
- `AudioDirector` : 2 lecteurs de musique en fondu, 6 voix d'effets non spatialisées.
  `battle_music.gd` (B3) : intensité par état, couches `sword_clash` / `march_drum`, stingers.
- Aucun `AudioStreamPlayer3D` dans tout le jeu : aucune spatialisation. `arrow_volley.ogg` et
  `gallop.ogg` existent mais ne sont référencés nulle part.
- Manques : ambiances de carte (vent, campagne, mer, ville), cris/clameur de bataille, chocs
  d'armes par régiment, volées de flèches, charge de cavalerie, météo (pluie, vent, tonnerre),
  feu et effondrement de murailles (S1/S2 muets), variations aléatoires de hauteur/volume.

## 4. Robustesse
- `cargo clippy --all-targets -- -D warnings` : propre (2 min 14 s à froid).
- `uv run --project tools pytest` : 275 réussis, **1 échec**
  `tools/tests/test_portraits.py::test_dry_run_makes_no_network_call` — test non hermétique :
  il dépend de l'état de `game/assets/portraits` (tous les portraits existent, donc « 0 portrait »
  à générer et pas de ligne « Style »). 4 avertissements Pillow (`mode=` déprécié, retrait Pillow 13
  le 2026-10-15) dans `tools/cent_ans_tools/geo/terrain.py:133`.
- `cargo test` : **435 réussis, 0 échec, 8 ignorés** (7 min 9 s en debug, machine partagée).
- Smoke Godot (`godot --headless --path game --script res://tests/smoke.gd`) : **exit 0**,
  249 s, aucune erreur de script ; 2 avertissements attendus (fixtures sans données → mock).
  Le smoke tourne en grande partie sur `CampaignSimMock` (1 766 lignes de GDScript qui
  reproduisent l'API de la simulation) : dette, risque de divergence avec le vrai cœur.
- Avertissements Godot en jeu : 35 × RGBFloat et 5 × RGB8 convertis (voir 2.3) ; en `--verbose`,
  une `StringName` orpheline (`PLAIN`) à la sortie de bataille.
- Fichiers énormes : `game/scripts/map/campaign_map.gd` 1 343 l., `battle_terrain.gd` 1 161,
  `battle_scene.gd` 1 141, `battle_meshes.gd` 1 058, `encyclopedia.gd` 1 054 ;
  `smoke.gd` 2 138 l. (un seul fichier de test pour tout le jeu) ; Rust `sim-battle/src/sim.rs`
  2 383, `diplomacy.rs` 2 334, `agents.rs` 1 904. Données : `data/map/heightmap.png` 14 Mo,
  `splat.png` 3,8 Mo versionnés en clair (pas de LFS). `.git` = 323 Mo, dont beaucoup de
  captures PNG de 1,5-1,9 Mo dans `docs/img/`.
- Hygiène du dépôt : `core/target` 12 Go et `core/target-merge` 2,8 Go (non suivi, hors
  `.gitignore`) ; `.uid` non suivis dans `game/tests/` ; `game/tests/c7_screenshot.gd` et
  `game/shaders/road_line.gdshader` sans `.uid`.
- Pas de banc de performance automatisé ni de seuil en CI : les bancs existants
  (`vegetation_bench.gd`, `--benchmark`, `--fps-probe`) sont manuels et bruités.

## 5. Lots proposés (classés par rapport impact / effort)

| # | Lot | Fichiers | Effort | Impact |
|---|---|---|---|---|
| T1 | **Optimiser la lib debug** : `[profile.dev.package."*"] opt-level = 2` (ou `opt-level = 1` pour les crates du workspace) ; fin de tour ×10-50 plus rapide dans l'éditeur, `cargo test` bien plus court. Alternative : profil `dev-opt` utilisé par `core/build.sh` | `core/Cargo.toml`, `core/build.sh` | S | 5 |
| T2 | **Relief fin adaptatif** : `fine_step` 2 au-delà de d≈80, 1 en deçà ; ombres du relief fin coupées au-delà de d≈120 | `game/scripts/map/terrain_builder.gd`, `campaign_map.gd` | S | 4 |
| T3 | **Audio de bataille spatialisé** : pool d'`AudioStreamPlayer3D` (chocs de mêlée par régiment en contact, volées avec `arrow_volley`, charges avec `gallop`, cris de déroute), bus `Ambiance`/`Bataille` dans un `default_bus_layout.tres`, limiteur sur Master, variations de hauteur | `game/scripts/audio/`, `battle_effects.gd`, `battle_scene.gd` | M | 5 |
| T4 | **Banque sonore libre** (CC0, voir A4) : 15-25 effets (ambiances campagne/mer/ville/vent, pluie, tonnerre, feu, effondrement, clameur, cris) + 2-3 musiques de plus (bataille, siège, paix) | `game/assets/audio/`, `docs/credits` | M | 4 |
| T5 | **Bond visuel à bas coût, bataille** : SSIL (qualité moyenne) + brouillard volumétrique léger par météo (brume au sol, pluie), `FogVolume` sur la fumée d'incendie ; campagne : glow léger et ciel `PhysicalSkyMaterial`/ciel de bataille réutilisé | `battle_atmosphere.gd`, `campaign_map.tscn` | S-M | 4 |
| T6 | **Eau de campagne** : normal map animée + réfraction/profondeur écran (reprendre `battle_water.gdshader`), reflet du ciel ; les côtes et fleuves sont le point faible visible au zoom comté | `game/shaders/water.gdshader` | M | 3 |
| T7 | **Ombres** : atlas 8192 → 4096 + `soft_shadow_filter_quality=2` hors réglage « Ultra » ; préréglages graphiques (bas/moyen/haut/ultra : ombres, SSAO, MSAA, densité de végétation) dans le menu Réglages | `project.godot`, `game/scripts/ui/settings*.gd` | M | 3 |
| T8 | **Banc de perf fiable** : temps d'image médian et p95 (pas les i/s), délai de garde et code de sortie ≠ 0 sans résultat, sortie JSON, script `tools/` qui enchaîne campagne (3 zooms) + bataille (48/80/120 régiments) + fin de tour, résultats consignés dans `docs/perf/` | `game/tests/perf_bench.gd`, `tools/` | M | 3 |
| T9 | **Fin de tour hors fil principal** (release) : `end_turn` dans un `WorkerThreadPool` ou un fil Rust, écran « tour des adversaires » animé ; puis profiler le pic à 18 s (probablement IA / événements, cf. `f7_events.rs` 134 s) | `godot-bridge/src/campaign_sim.rs`, `campaign_map.gd` | M-L | 3 |
| T10 | **Imposteurs d'arbres lointains et occlusion** : billboard octaédrique pour la végétation de campagne au-delà du LOD ≈ 20 triangles ; `OccluderInstance3D` sur les murailles et le relief en bataille | `vegetation*.gd`, `battle_siege.gd` | L | 2 |
| T11 | **Dette** : corriger `test_portraits.py` (données factices au lieu de l'état de `game/assets/portraits`), `mode=` de Pillow avant le 15/10, `.uid` manquants, `core/target-merge` dans `.gitignore`, découper `smoke.gd` par domaine, réduire `CampaignSimMock` au strict nécessaire | `tools/tests/`, `tools/cent_ans_tools/geo/terrain.py`, `.gitignore`, `game/tests/` | S | 2 |
| T12 | Localiser et supprimer les conversions RGBF → RGBAF au chargement (créer directement en RGBAF/RGBH ou en R16) | `game/scripts/map/*` | S | 1 |

Ordre conseillé : T1 et T2 tout de suite (quelques lignes, gains mesurés), T3+T4 ensemble
(la bataille est muette hors musique), puis T5, T8 et T7.
