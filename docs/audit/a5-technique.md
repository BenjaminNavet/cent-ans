# Audit A5 — technique (rendu, performance, audio, robustesse)

État : en cours (reprise après crash machine). Auditeur A5, session de nuit 7, 2026-09-24.
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
(à compléter)

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
- (reste à compléter)

## 5. Lots proposés (classés)
(à compléter)
