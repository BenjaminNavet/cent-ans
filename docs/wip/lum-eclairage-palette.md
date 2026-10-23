# LUM : éclairage, ciel et palette unifiés campagne / bataille

Branche `worktree-agent-abf125ec26f3d1ce0`. Source : `out/juge/iter-02-art.md` (constats 3, 4, 5, 8, chantier 3). ADR 0755.

## Plan
1. Bible de palette : `docs/design/palette.md` + `data/fx/palette.json` (+ schéma) ; saison de référence printemps ; test de cohérence avec `atmosphere.json`.
2. Bataille : 6 clés d'heure (dawn, morning, midday, afternoon, dusk, night) avec soleil, étalonnage, brume colorée ; cause des taches du dusk ; ciel plus présent.
3. Campagne : même étalonnage commun (saturation, contraste), voile retiré en météo claire (`campaign_post.json`), brouillard (ADR 0596) intact.
4. Herbe de bataille : macro-variation de teinte pilotée par bruit monde (`ground.wesl`).

## État
- [ ] squelette (ce fichier)

## État final (arrêt demandé)
Fait : palette.json/md + schéma, `palette.rs` (sol de bataille lu de la palette), six heures (soleil en K, air par heure, ciel dawn), bosses du sol atténuées en lumière rasante (cause des taches), macro-variation d'herbe, étalonnage commun campagne/bataille, voile de campagne réduit (`clear_scale`), ADR 0755.
Vérifié : tests ciblés 113/114 (le 114e, `grades_compose_like_the_lut`, corrigé à la lecture, NON relancé) ; pytest schémas OK ; 8 captures de bataille (aube, midi, crépuscule : propres, distincts). Campagne : captures inexploitables (terrain non chargé en debug sous charge), seulement données + tests.
Non fait / à faire : clippy -D warnings, smoke, bench fps (un tap de bruit de plus au sol), jaune des couches de texture (teinte 67°), interpolation du soleil entre phases, horizon/panorama/volumétrique non teintés par l'heure, ciel en vue haute.
