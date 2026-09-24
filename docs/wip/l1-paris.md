# L1 — Villes emblématiques : Paris d'abord

Branche `worktree-agent-a36a911aec4fbbb5c` (depuis `main` 93d466c). Backlog : `docs/audit/backlog-tw.md`
§ « Villes emblématiques ». ADR : `docs/decisions/0015-landmark-cities.md`. Captures : `docs/audit/captures/l1/`.

## État : terminé (à fusionner par l'orchestrateur)

- [x] Gabarit : plan `data/landmarks/<id>.json` (schéma `data/schemas/landmark.schema.json`), coordonnées
  locales en mètres réels (x est, y nord vrai, origine `anchor.lonlat` = Notre-Dame). Test
  `tools/tests/test_landmarks.py` (schéma, ancrage px et nord EPSG:3035, loupe monotone, références du siège).
- [x] Générateur Blender en ligne de commande : `tools/blender_scripts/landmark_city.py` (loupe radiale,
  Seine, îles, enceintes datées, ponts habités, rues, tissu Voronoï de ≈ 5 000 maisons, îlots LOD),
  `landmark_geometry.py` (primitives sans bpy), `landmark_monuments.py` (Notre-Dame soignée, Sainte-Chapelle,
  palais de la Cité, Louvre de Philippe Auguste et de Charles V, Châtelets, Bastille, Halles, églises,
  abbayes, Temple, tour de Nesle, moulins de Montmartre), `landmark_preview.py` (rendus EEVEE de contrôle).
- [x] Plan recalé sur OpenStreetMap (berges, île de la Cité, île Saint-Louis coupée au canal, vestiges des
  enceintes, monuments) ; sources du domaine public citées dans le JSON (plan de Bâle, Legrand 1380,
  Viollet-le-Duc ; ALPAGE CC BY consulté).
- [x] Carte de campagne : `LandmarkModel` + `landmark.gdshader` (drapé sur le relief affiché par ancrage
  UV, hauteurs cuites 96², recuites quand une tuile change), `LandmarkLibrary` ; `SettlementLayer`
  remplace la maquette générique de `set_paris`, masque hameaux et arbres dans la zone ; couches datées
  (`charles_v` dès 1358, `bastille` 1370, `pont_saint_michel` 1378, `louvre__charles_v` 1364) selon
  l'année du libellé de date ; LOD maisons < 120 / îlots au-delà ; zoom caméra jusqu'à 7 au-dessus de
  Paris (`CampaignCamera.close_zones`, `close_min_distance`).
- [x] Bataille de siège dans l'Île-de-France : `LandmarkBackdrop` (`game/scripts/battle/landmark_backdrop.gd`)
  pose `paris_siege.glb` (échelle réelle : Seine, Cité, Notre-Dame, rive droite, Louvre) derrière la ville
  assiégée générique de la simulation. Capture : `--landmark-backdrop=paris`.
- [x] Captures : `avant_campagne_d24/d70`, `apres_campagne_d9/d24/d70/d250`, `apres_notre_dame_d7`,
  `apres_1380_d12` (`--landmark-year=1380`), `blender_*` (Notre-Dame façade et flanc sud, plan),
  `avant_siege_large`, `apres_siege_paris(_large)`.

## Point d'accroche V4 (fleuves) — à reporter à la fusion
V4 lit `data/map/river_styles.json` → `custom_zones`. Valeurs L1 :
`{ "id": "paris", "name": "Paris", "lonlat": [2.3499, 48.853], "radius_px": 6.8, "boundary_bridges": false }`
(V4 proposait `[2.3488, 48.8534]`, 4,5 px). Tant que ce n'est pas fait, le ruban générique de la Seine
passe sous la maquette (visible entre les îlots).

## Régénérer
```
blender --background --python tools/blender_scripts/landmark_city.py -- data/landmarks/paris.json game/assets/models/landmarks/paris.glb
blender --background --python tools/blender_scripts/landmark_city.py -- data/landmarks/paris.json game/assets/models/landmarks/paris_siege.glb --siege
godot --headless --path game --import
```

## Points ouverts
- La ville assiégée reste la ville générique de la simulation (murailles, maisons-obstacles) : un siège
  « dans » le plan de Paris demanderait une géométrie de siège lue des données dans `core/` (sim-battle).
- GLB lourds (paris ≈ 10 Mo / 110 k triangles, paris_siege ≈ 9 Mo / 104 k).
- Mesures (M4 Pro partagé, vsync) : d = 24 sur Paris 50 i/s contre 32 i/s sur Orléans au même moment ;
  pas de surcoût mesurable, bruit de la machine partagée.
- Montmartre : abbaye et moulins posés à plat (le relief de la carte est drapé mais peu marqué).
- Suite du gabarit : Londres, Avignon, Calais, Bordeaux, Rouen, Bruges (un JSON + gabarits de monuments).
