# TB2 — désencombrement et lecture « à la ToB »

Worktree `../gp-tb2`, branche `feat/tb2`. Plan : `docs/design/2026-10-02-campagne-tob.md` § 3.
Rendu/UI Godot seulement, `core/` intact.

## État
- [x] Squelette du test `game/tests/tb2_declutter_test.gd`
- [x] 1. Brouillard de guerre : voile de parchemin (bloc brouillard de `terrain.gdshader`, réglages
      `fog_of_war` de `data/ui/campaign_map.json`, lus par `game/scripts/map/map_readability.gd`).
      Mesure `ss_shot.gd --stats --at=1930,2560 --distances=300` (Midlands vus de fac_france),
      moyenne RVB / écart-type : sans brouillard 82 83 39 / 41 33 24 ; avant 126 124 108 /
      40 37 38 (nappe blanche) ; après 88 76 49 / 31 27 21 (sépia, à peine plus sombre que le sol vu).
- [ ] 2. Frontières et tracés moins saturés, plus fins hors sélection / survol / diplomatie
- [ ] 3. Pictogrammes : un signe par ville et par palier ; rosaces et étoiles en mode de carte
- [ ] 4. Étiquettes : hiérarchie, pas de nom coupé, noms de région en vue moyenne
- [ ] 5. Nuages et brumes : météo réelle seulement, plus fins, jamais sur la province sélectionnée

## Prochaine étape
Point 2 : style « au repos » des frontières FR1 (`faction_borders.gdshaderinc`, bloc `rest` de `data/map/faction_borders.json`).
