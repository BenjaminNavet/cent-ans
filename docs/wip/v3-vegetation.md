# WIP — lot V3 « Végétation, villes et marqueurs de la carte de campagne »

Branche `visual-v3` (depuis `visual`, V1 + V2 fusionnés). **État : terminé**, à fusionner par l'orchestrateur.
Captures : `docs/img/visuel/v3_far.png`, `v3_mid.png`, `v3_forest.png`, `v3_city.png`, `v3_army.png`,
`v3_army_states.png` (au repos sélectionnée / en marche « » » / siège / à bord).

## Fait
- **Végétation** (`game/scripts/map/vegetation*.gd`, `game/shaders/foliage.gdshader`, nœud `Vegetation`
  de `campaign_map.tscn`, autonome : lit `map_data`/`load_ok` du parent et `CameraRig`, aucun appel dans
  `campaign_map.gd`). Tuiles 16×16 (4 parties chacune) semées dans `WorkerThreadPool`, cache LRU
  (64 tuiles), démarrage à chaud (tuiles proches attendues au premier affichage, ≈ 0,4-1 s).
  Feuillus (130 tri) / conifères (altitude, latitude) + variantes lointaines ≈ 20 tri, teinte / taille /
  rotation / inclinaison par instance, vent (vertex), bruit de feuillage, éclaircissement par graine
  (`visible_instance_count` sur tampons triés), densité réduite au dézoom, plus d'arbres au-delà d'une
  distance caméra de 700. Ombres des parties proches seulement. Haies (parcellaire déformé, denses
  dans le bocage), bosquets, arbres isolés. Masque : `MapData.splat_image` (B forêt, G + 0,6 R campagne),
  repli procédural (terrain des provinces, altitude, pente, bruit) si la splat manque.
- **Villes** (`tools/blender_scripts/models.py`) : villes fortifiées entières (murs irréguliers, tours,
  portes, église / cathédrale, maisons alignées), `city_cathedral` (nouveau), village ouvert ; matériaux
  PBR (couleur, rugosité, métal) de teintes naturelles ; fondations sous z = 0 (pas de flottement en
  pente) ; `CITY_SCALE` 6,5 → 4,8.
- **Armées** : groupe de figurines (chef monté + 2-4 d'escorte selon l'effectif et la composition :
  `army_foot`, `army_archer`, cavaliers ; catégories lues dans `data/unit_types/`), orientées vers la
  prochaine étape ; étendard `map_banner.gdshader` (flotte, billboard vertical) : oriflamme / St George pour
  l'armée du roi, `heraldry/banners/<id>_banner.png`, repli sur l'écu rogné puis la couleur ; anneau au
  sol (`Decal`, couleur de faction, doré si sélectionné) ; plaque d'effectif 2D (écu + hommes + état) de
  taille constante ; plus d'ombre portée au-delà de l'échelle 2,6 ; hampe et drapeau sans ombre.
- Camp de siège et cogue refaits ; l'étendard passe au mât de la cogue.
- Outils : `game/tests/vegetation_bench.gd` (banc), `game/tests/v3_markers_stage.gd` (mise en scène).

## Performance (M4 Pro, 1440×900, vsync coupée, autres agents actifs)
Avant V2 (terrain V1) : dézoom 5 ms, zoom moyen 9 ms, proche 8-9 ms, très proche 7 ms, bocage 11 ms
(sans végétation : 8,3 ms de près). 2-5 M primitives, semis ≈ 250-400 ms / tuile hors fil principal.

## Points ouverts
- Flammes (`*_pennon.png`) et `dragon.png` non utilisées ; `standard_for` sait afficher une flamme (mode 1).
- Banc non relancé après la fusion V2 (machine chargée) ; captures `v3_mid`/`v3_forest` faites juste
  avant le dernier réglage des anneaux.
- Le smoke test ne vérifie pas encore la végétation ni les plaques (fichier hors périmètre).
