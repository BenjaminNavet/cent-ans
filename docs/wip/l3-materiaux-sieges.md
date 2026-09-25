# L3 — Villes emblématiques : matériaux et sièges

Branche `worktree-agent-af2974d71a33b3dc3`. Suite de L1 (`docs/wip/l1-paris.md`) et L2
(`docs/wip/l2-villes.md`), ADR 0015. Captures : `docs/audit/captures/l3/`.

## État
- [x] Atlas partagé `game/assets/textures/landmarks/` (Texture2DArray 16 couches 512², albédo « détail »
  + normales ; Poly Haven CC0 + couches procédurales : pans de bois, plomb, vitrail, sol, masques de
  vieillissement), `build_textures.py`. Réglages globaux `shader_globals` (project.godot) : aucun
  script qui pose le shader n'a besoin de changer (menu MM1 compris).
- [x] `landmark.gdshader` : triplanaire local, couleur de palette × détail, variation par bâtiment
  (hachage de l'ancrage), grandes nuances, pied des murs, coulures, plaques, mousse sur les faces au
  ciel ; normales plates dérivées (plus de NORMAL dans les GLB) ; `meters_per_unit`, `ground_height`.
- [x] Générateur : alpha de la teinte = couche × 16 + pas d'exagération (`material_code`), bloc
  `materials` par ville (murs et toits des maisons, palette : calcaire blond, Caen, pierre d'Avignon,
  brique de Bruges), `drop_normals` → Paris 5,7 Mo (était 10), toutes les villes ≤ 6 Mo.
- [x] Toiles de fond de siège des 6 villes L2 (bloc `siege` : `cut_m`, `center_x_m` nouveaux), hauteurs
  cuites au maximum du texel dans `LandmarkBackdrop`.
- [ ] Siège dans le plan : `siege.battle` (enceintes, porte attaquée, rues) écrit dans les 7 JSON ; reste
  le cœur (`SiegeLayout`, ADR 0026), le pont et le rendu des châtelets/rues.
- [ ] Captures finales (menu, campagne, sièges des 7 villes).

## Prochaine étape
Cœur : `data-model` charge `data/landmarks/*.json` (sous-ensemble siège), `sim-campaign`
(`battle_request.rs`) joint le `SiegeLayout` au `SiegeSetup`, `sim-battle::siege_layout` construit
`SiegeWorks` (anneau ramené à l'échelle, porte face à −z, châtelets, maisons hors des rues).

## Outils
- Gros plan : `godot --path game --script res://tests/l3_landmark_shot.gd -- --model=paris_siege
  --cam=x,y,z --target=x,y,z --out=<png> [--flat] [--debug=1..5]`.
- Régénérer : `blender --background --python tools/blender_scripts/landmark_city.py --
  data/landmarks/<id>.json game/assets/models/landmarks/<id>[_siege].glb [--siege]`.
- Atlas : `uv run --no-project --with pillow --with numpy python
  game/assets/textures/landmarks/build_textures.py <téléchargements Poly Haven 1k>`.
