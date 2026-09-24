# Lot CV2 — Armées et flottes figurées sur la carte de campagne

Branche `worktree-agent-a03415e18588b6fde`. Rendu seulement : aucune règle, `movement.rs`
intact ; lecture de l'état et du chemin via le pont existant (`get_army`, animation M4
`ArmyMarkers.place_marker`).

## État : implémentation en cours (première version écrite, pas encore lancée)
- `game/scripts/map/army_figures.gd` (`ArmyFigures`) : figurines skinnées V2 (texture d'os)
  d'une armée : général monté (`cavalry_0`), porte-étendard (`infantry_0`), escorte de 2 à 6
  soldats selon l'effectif, répartie selon la composition (catégorie + `BattleMeshes.variant_of`,
  comme en bataille) ; états marche / repos ; camp de siège ou bivouac (rase campagne, à
  l'arrêt) avec fumée ; flottes (1 à 3 cogues/nefs) qui tanguent ; fondu au palier « loin ».
- `game/scripts/map/army_marker.gd` : branchement, la hampe suit le porte-étendard.
- `game/scripts/map/army_markers.gd` : `figure_weight` (ZoomTiers), `set_view`, marche pendant
  `place_marker`.
- `game/shaders/campaign_sail.gdshader` : voile aux couleurs et armes de la faction.
- `tools/blender_scripts/campaign_fleet.py` → `game/assets/models/fleet/{cog,nef,bivouac}.glb`.
- `game/tests/cv2_armies_stage.gd` : mise en scène, captures, mesure de FPS
  (`--legacy-army-markers` = avant CV2).

## Prochaine étape
Import Godot, captures avant (`--legacy-army-markers`) / après, réglages d'échelle, FPS.
