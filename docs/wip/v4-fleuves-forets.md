# V4 — Fleuves, ponts et forêts de la carte de campagne (lots A1-11, A1-10)

Branche `worktree-agent-ae5685646cc8b59ac` (depuis `main` 356a1ad). Rendu seulement.
Captures : `docs/audit/captures/v4/` (avant `avant_*`, après `apres_*`).
Scripts de capture / banc : `scratchpad/v4/cap.sh`, `scratchpad/v4/fps.sh` (copies dans ce fichier en fin de lot).

## Plan
### A1-11 fleuves et ponts
1. Outil `tools/cent_ans_tools/geo/river_render.py` (`cent-ans geo rivers-render`) :
   - lit `rivers.geojson`, `crossings.json`, `river_styles.json` (largeurs par fleuve, en données) ;
   - oriente chaque tronçon dans le sens de l'écoulement (altitude), largeur croissante vers l'aval ;
   - écrit `data/map/rivers_render.json` (tronçons : points, largeurs, importance) ;
   - écrit `data/map/river_bed.png` (4096², L8 : distance signée à la berge) pour le shader de terrain ;
   - écrit `data/map/crossings_px.json` (ponts, gués, bacs : position px recalée sur le fleuve,
     direction, largeur, structure pierre/bois/bateaux/bac/gué).
2. `terrain.gdshader` : lit creusé (sommets abaissés de près), berges (boue, herbe grasse), normale
   des berges depuis le gradient de `river_bed`.
3. `rivers_renderer.gd` + `river_water.gdshader` : ruban d'eau avec test de profondeur (plus de
   fleuve dessiné par-dessus les murs), écoulement, reflets, eau peu profonde en bord.
4. `river_crossings.gd` : ponts 3D procéduraux (pierre à arches, bois sur pilotis, bateaux), bacs,
   gués (pierres, remous) ; ponts aux murs des villes traversées par un fleuve (Paris).
### A1-10 forêts
5. Essences : chênaie, hêtraie, conifères de montagne, bocage/haies ; maillages procéduraux
   distincts ; canopée continue (masse par tuile + arbres de lisière) ; teinte saisonnière.

## État
- [ ] squelette
