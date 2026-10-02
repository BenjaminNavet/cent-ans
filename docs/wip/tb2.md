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
- [x] 2. Frontières FR1 au repos (bloc `rest` de `data/map/faction_borders.json` : largeur × 0,6,
      opacité × 0,6, halo × 0,25, saturation 0,45) ; pleine intensité pour les royaumes des provinces
      sélectionnée et survolée (`fr1_focus_a/b`), en mode Diplomatie (`modes.diplomacy.full`) et sur
      le parchemin (`rest.parchment_focus`). Chemins d'armée adoucis (déjà réservés à la sélection).
- [x] 3. Pictogrammes (ADR 0151) : écu seul signe d'une ville, réservé aux lieux de rang 3-4 au-delà
      de 330 (rang 1 : 220) ; marteaux, sceaux d'incident et sites de rencontre réservés au calque
      « Signes » du menu des filtres, à certains modes de carte et aux cas pressants. Mesure à Paris,
      écran 1280×720 : écus 25 / 20 / 38 aux distances 1100 / 400 / 90 (45 à 400 avant), 0 autre signe.
- [ ] 4. Étiquettes : hiérarchie, pas de nom coupé, noms de région en vue moyenne
- [ ] 5. Nuages et brumes : météo réelle seulement, plus fins, jamais sur la province sélectionnée

## Prochaine étape
Point 4 : étiquettes (petites capitales pour les capitales, pas de nom coupé, noms de région en vue moyenne).

## Points ouverts
- `da7d_overlap_test.gd` échoue sur son seuil de temps (4 ms) quand la machine est chargée : 5,4 ms avant TB2, 4,4 ms après, charge moyenne > 10. À relancer machine calme.
