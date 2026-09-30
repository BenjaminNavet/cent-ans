# SS — sol « satellite » de la campagne

Spec : `docs/superpowers/specs/2026-09-30-ss-sol-satellite-design.md` ; ADR 0141. Autonomie totale (joueur 30/09).

## État : fusionné dans main, en attente du jugement du joueur
- [x] SS1 `cent-ans geo colormap` : BC1 14336×12288 + mipmaps, 4 parts (~46 Mo), cuisson ~2 min 20 ; style `data/map/colormap_style.yaml` ; aperçu `data/map/colormap_preview.jpg` ; note `docs/wip/ss1-colormap.md`.
- [x] SS2 shader : `satellite_ground.gdshaderinc` (crochet après `fp_parcels`), chargement `ReliefLandcover.load_colormap`, repli sans carte ; routes principales en terre (`road_renderer.gd`), estompées de loin avec la carte ; relief moins exagéré de près (`shading_relief_near` 1,3).
- [x] **Cause principale du « relief IGN » trouvée** : le brouillard matinal météo (`weather_ground`) peignait le sol en gris clair jusqu'à 80 % dans les basses terres (tout le bassin parisien). Plafonné à `weather_mist_max` 0,15 et teinté par le sol. Mesure (Paris, distance 90, été) : sol rendu 125/112/91 (beige gris) → 108/95/71 ; avant la météo 94/81/55.
- [x] SS3 lacs : `cent-ans geo lakes` → `data/map/lakes.json` (762 nappes, 67 nommées), `lakes_renderer.gd`, mode `sheet` de `river_water.gdshader`.
- [ ] SS4 fal.ai : **reporté**, pas nécessaire au vu des captures ; à rouvrir si le joueur trouve le détail proche trop flou (textures chaume/labour/vigne/grève).
- [x] SS5 tests : smoke 30/30, `ss_colormap_test`, `ss_lakes_test`, `cv1_campaign_life_test`, 20 pytest. Banc : toutes vues au plafond 60 Hz (vsync macOS) avec et sans carte : pas de régression visible ; banc GPU réel sur machine calme à faire (comme `fps-carte.md`).

## Outils de diagnostic
`game/tests/ss_shot.gd` : captures par distance, `--stats` (couleur moyenne sans lire l'image), `--param=`, `--env=`, `--hide=`, `--bench`, `--season=`.

## Points ouverts
- Détail proche encore un peu flou (parcelles procédurales × teinte de la carte, `sg_near_keep` 0,45).
- Bosses du relief de près : viennent surtout de l'exagération verticale (géométrie), pas de l'ombrage.
- 5 retenues modernes encore en eau dans `land_mask.png` (Sainte-Croix, Der, Orient, Ebro, Riaño) : peintes en terre dans la carte mais l'eau peinte du shader peut rester.
- Neige hivernale de plaine (CV1) inchangée.
