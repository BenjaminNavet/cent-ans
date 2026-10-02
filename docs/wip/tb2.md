# TB2 — désencombrement et lecture « à la ToB »

Worktree `../gp-tb2`, branche `feat/tb2`. Plan : `docs/design/2026-10-02-campagne-tob.md` § 3.
Rendu/UI Godot seulement, `core/` intact.

## État
- [x] Squelette du test `game/tests/tb2_declutter_test.gd`
- [ ] 1. Brouillard de guerre : voile de parchemin (bloc brouillard de `terrain.gdshader`)
- [ ] 2. Frontières et tracés moins saturés, plus fins hors sélection / survol / diplomatie
- [ ] 3. Pictogrammes : un signe par ville et par palier ; rosaces et étoiles en mode de carte
- [ ] 4. Étiquettes : hiérarchie, pas de nom coupé, noms de région en vue moyenne
- [ ] 5. Nuages et brumes : météo réelle seulement, plus fins, jamais sur la province sélectionnée

## Prochaine étape
Mesure « avant » du brouillard (`ss_shot.gd --stats` sur l'Angleterre vue de fac_france), puis point 1.
