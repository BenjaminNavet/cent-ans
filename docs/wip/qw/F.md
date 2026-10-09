# QW-F : fumée brun-rouge près des foyers

État : fait. Cause : fumée de siège trop sombre et chaude (couleur de base 0,26/0,25/0,235 + lumière orange des foyers), dense dès la naissance.
- `game/shaders/fire_smoke.gdshader` : uniforms `age_darken`, `young_density`, `young_span` (défauts neutres : campagne et bombarde inchangées).
- `data/fx/siege_fire.json` (bloc smoke) : couleur gris-bleuté 0,52/0,55/0,60, `age_darken` 0,72, `young_density` 0,55, `point_light_share` 0,08 ; schéma mis à jour ; câblage dans `siege_fire_fx.gd`.
- Vérification : `game/tests/qwf_smoke_shot.gd` (A/B sans noyau Rust). Le noyau actuel est incohérent (data/naval/rules.json `crew_ammo_cap` inconnu de la dylib), donc `smoke.gd` échoue pour une cause étrangère au lot.
- Reste : revoir en siège réel une fois la dylib reconstruite (`core/build.sh`).
