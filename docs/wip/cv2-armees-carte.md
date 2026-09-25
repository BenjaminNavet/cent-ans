# Lot CV2 — Armées et flottes figurées sur la carte de campagne

Branche `worktree-agent-a03415e18588b6fde` (main fusionné le 2026-09-25 : L1, M3, R1… sans
conflit ; dylib reconstruite, import et smoke verts). Rendu seulement : aucune règle,
`movement.rs` intact ; lecture de `get_army` et de l'animation M4 (`ArmyMarkers.place_marker`).

## État : terminé (non fusionné)
- `game/scripts/map/army_figures.gd` (`ArmyFigures`, nœud `M10Model`) : figurines skinnées V2
  (maillages `battle_skinned`, texture d'os, un `MultiMesh` et un matériau par figurine) :
  général monté (`cavalry_0`), porte-étendard (`infantry_0`, la hampe suit sa main quand la
  troupe tourne), escorte de 2 à 6 soldats (≤ 300 / 800 / 1 500 / 3 000 hommes / au-delà)
  répartie au plus fort reste selon la composition (catégorie + `BattleMeshes.variant_of`, mêmes
  figurines qu'en bataille : archers anglais, génois, piquiers flamands, milice…), cavaliers
  devant, tireurs derrière. Livrée et armes de la faction (`livery`, `heraldry`).
- Animation : `idle`/`guard` à l'arrêt, `walk` (ou `c_walk`) pendant l'animation M4 du
  déplacement (`place_marker` → `set_walking`), fondu du shader V2 entre les deux ; phase propre
  à chaque armée.
- Camp : siège → `siege_camp` (agrandi) + fumée ; armée en rase campagne à l'arrêt → bivouac
  (pavillon rayé aux couleurs, deux tentes, feu) + fumée, seulement sous 190 de distance caméra.
  Fumée = `fire_smoke.gdshader` + planche V3 (10 particules, coupée au-delà).
- Flotte (armée embarquée / en mer) : 1 à 3 navires (1 par 900 hommes), cogue et nef alternées
  (`tools/blender_scripts/campaign_fleet.py` → `game/assets/models/fleet/{cog,nef,bivouac}.glb`,
  618 / 666 / 438 triangles), voile `campaign_sail.gdshader` (couleur, laizes, armes au centre,
  creux qui respire), tangage/roulis/pilonnement déphasés ; étendard sur une hampe de poupe.
- Zoom : figurines LOD0 < 90, LOD1 < 320, LOD2 au-delà ; au palier « loin » (`ZoomTiers.far_weight`)
  elles rétrécissent et disparaissent dans l'étendard : restent drapeau + plaque d'effectif (V3).
- Repli A/B : `--legacy-army-markers` (après `--`) = figurines M10/V3 d'avant.
- Smoke : contrôle CV2 (6 figurines pour 1 500 hommes, bivouac, 2 navires).

## Captures (`docs/audit/captures/cv2/`, `game/tests/cv2_armies_stage.gd`)
`avant_{60,160,320,800}` (legacy) / `apres_{35,60,90,160,200,320,800}`, `apres_armee_28`
(gros plan, bivouac), `apres_flotte_30` (cogue bretonne), `apres_marche_40` (marche, après fusion).

## Performance (M4 Pro partagé avec d'autres agents, 1600×900, sans vsync, 300 images)
`--fps --crowd` : 40 armées toutes visibles (5 en siège), deux passes alternées :
| Distance | CV2 (i/s) | avant (i/s) | primitives |
|---|---|---|---|
| 150 | 32,1 · 28,0 | 28,3 · 24,8 | 14,42 M contre 14,01 M (+3 %) |
| 320 | 52,5 · 57,6 | 38,1 · 39,2 | 2,05 M contre 2,01 M |
Aucune baisse mesurable (le bruit de la machine domine ; CV2 est même devant : moins de nœuds
que les glb M10 par armée). Objectif « pas plus de −10 % » tenu.

## Points ouverts
- La hampe de l'étendard est fixe (ne suit pas le balancement de la marche) ; celle de la
  flotte ne suit pas le tangage.
- Pas de « posture défensive » dans la simulation : le bivouac s'affiche pour toute armée à
  l'arrêt en rase campagne (hors colonie).
- Pas de figurines sur le pont des navires ; pas de sillage.
- Armées IA non animées pendant le tour IA (limite M4) : elles n'apparaissent en marche que
  si l'animation M4 les déplace.
- Glissement des pieds (cadence de marche fixe, comme en bataille).
