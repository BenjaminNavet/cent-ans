# EP2 — Horizon des batailles

Branche : `worktree-agent-a5e4208be5556aa24`. ADR : `docs/decisions/0032-horizon-de-bataille.md`.
Plan d'ensemble : `docs/wip/epic.md`.

## Conception retenue
- **Relief réel** : `cent-ans geo horizon` cuit, par province (centroïde, ou point à 3-5 km de la
  mer ouverte pour une province côtière), une tuile 26 × 26 km à 100 m (Copernicus GLO-90 moyenné,
  repli tuiles fines 360 m hors emprise), altitude int16 (quart de mètre) + classe (forêt de
  `splat.png`, mer) + profil de ligne d'horizon réel 12-150 km sur 256 azimuts, dégonflés dans
  `game/assets/horizon/relief/<prov>.bin` (132 tuiles, 11 Mo) + `index.json`.
- **Raccord** : `BattleTerrain.world_height` → `BattleHorizon.blend` (0,7 → 3 km du bord du champ ;
  rivière 450 m et côte B5 gardent la main). Tuile tournée pour que la mer réelle tombe du côté du
  flanc côtier de la simulation. Bois lointains = forêts réelles.
- **Anneau d'horizon** 7 → 13 km (pas 400 m, `battle_horizon_land.gdshader`, brume `FOG` calée
  sur celle de la scène, allégée en altitude, neige en plaques au-dessus de la limite de saison),
  bord extérieur plongeant ; **mer lointaine** (`battle_sea` réutilisé).
- **Panorama** (`battle_panorama.gdshader`) : cylindre 14,6 km qui suit la caméra ; chaque colonne
  peinte est recalée sur la crête réelle de son azimut (`paint_detail` = part du relief peint
  gardée) ; secteurs de mer = horizon marin.
- **Silhouettes** : villages (église + maisons), château sur une hauteur, fumées (shader billboard).
- **Données** : `data/fx/horizon.json` + `data/schemas/fx_horizon.schema.json`.
- Drapeaux : `--no-horizon` (A/B), `--horizon-province=<id>`, `--panorama=<id>`, `--no-hud`
  (captures sans interface, `BattleScene._take_screenshot`).

## État
- [x] cuisson relief + index (`tools/cent_ans_tools/geo/horizon.py`, tests `tools/tests/test_horizon.py`)
- [x] panoramas : 13 bandes (sonde puis lot, Flandre régénérée), 0,57 $ consignés dans `docs/budget.md`
  (`cent-ans assets horizon-panoramas [--generate]`, originaux `tools/horizon_raw/`)
- [x] intégration Godot (anneau, mer, cylindre, silhouettes), test `game/tests/ep2_horizon_test.gd`
- [x] brume / fog (densité par météo dans `horizon.json`)
- [x] mesures A/B, captures `docs/img/ep2/` (before_/after_ : plaine, cote, montagne, alpes, hiver)
- [x] fusion de main (EP4 audio, playlists, UI1 enluminée, EQ1 équilibrage, UX1/UX2 ; un seul
  conflit, `docs/budget.md`, résolu en gardant les deux séries de lignes de dépense)

## Mesures (M4 Pro partagé avec d'autres agents, Vulkan, 1600 × 900, qualité Haute, 1 160 soldats)
`--benchmark --bench-repeat=2 --terrain=hills --horizon-province=prov_bearn [--camera=600,740,40,180]
[--no-horizon]`, temps GPU moyen (ms) :
- vue par défaut (plongeante) : horizon 9,30 / 9,31 ; sans 9,37 / 9,81 → pas d'écart mesurable ;
- vue rasante face aux Pyrénées : horizon 10,28 / 9,79 / 9,95 (une passe aberrante à 20,9 écartée,
  machine chargée) ; sans 10,77 / 9,22 / 9,06 → **+0,6 à +0,9 ms** au pire, dans le budget de 1 ms.
  Anneau d'horizon : 5 004 triangles ; panorama : 256 triangles ; silhouettes : quelques centaines.

## Fusion et vérifications (25/09)
- `git merge main` : un seul conflit (`docs/budget.md`), résolu en conservant les deux séries de
  lignes (EP2 panoramas + EP4 sons Freesound) sous la même section « Batailles épiques ».
  `tools/cent_ans_tools/cli.py` fusionné automatiquement.
- EP1 (taille de champ paramétrique) n'est pas encore dans `main` : `battle_terrain.gd` a toujours
  `FIELD_W`/`FIELD_D` en `const`. `BattleHorizon` reçoit déjà `field_size`/`centre` en paramètre de
  `load()` (pas de constante dupliquée côté horizon), donc rien à rendre paramétrique côté EP2 pour
  l'instant ; à revérifier quand EP1 fusionnera si `NEAR_RECT`/`FAR_RECT` deviennent des variables.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` :
  **OK** (Rust modifié par la fusion — retinue/ai/négociation/agents — pas par EP2 lui-même).
- `core/build.sh` : **OK** après un `cargo clean -p data-model -p godot-bridge` (un premier essai a
  échoué avec une erreur du linker sur un objet obsolète, `libdata_model-*.rlib` incohérent après le
  premier `cargo build -p godot-bridge` isolé — probablement un artefact incrémental corrompu par la
  fusion de nombreux commits Rust d'un coup ; le rebuild propre l'a résolu).
- `uv run --project tools pytest` : **OK**.
- `godot --headless --path game --import` : **OK**.
- `godot --headless --path game --script res://tests/smoke.gd` : **bloqué**, reproductible 2 fois de
  suite à l'identique. Le flux carte de campagne (`_run_campaign_map`) charge la simulation réelle
  (`smoke campaign: simulation REAL ... store loaded`), fait tourner `CampaignLife` un cycle, puis
  plante systématiquement sur `Failed method: Container::_sort_children. Message queue out of memory.`
  Aucun lien avec EP2 : le crash a lieu sur la carte de campagne, avant toute scène de bataille (le
  code d'horizon n'est même pas chargé). Ça sent une boucle de redimensionnement UI (thème enluminé
  UI1 + panneau de province EQ1 « vrai mécontentement ») entrée en résonance avec les vraies données
  de campagne. Hors périmètre EP2 (`game/scripts/map/*`, `game/scripts/ui/*` appartiennent à
  UI1/EQ1/UX1/UX2) — je n'y touche pas. À signaler à l'orchestrateur avant fusion dans
  `integration/epic` : le smoke complet de `main` fusionné est actuellement rouge pour une raison
  indépendante de l'horizon.

## Prochaine étape
Le code EP2 est fusionné, testé (Rust/Python) et prêt pour `integration/epic` du point de vue de
l'horizon lui-même. Le blocage restant (`Container::_sort_children`, carte de campagne) est un bug
pré-existant d'un autre lot fusionné (UI1/EQ1) à faire corriger avant de considérer le smoke général
vert. Ensuite (hors lot) : tuile au lieu exact de la bataille quand la campagne exportera une
position, tuiles des sites historiques pour EP7.

## Captures
`godot --path game --resolution 1600x900 res://scenes/battle/battle.tscn -- --deploy-shot
--screenshot=<png> --camera=<x,z,d,lacet> --no-hud --terrain=<…> --horizon-province=<id>
[--coast] [--season=winter] [--no-horizon]`.
