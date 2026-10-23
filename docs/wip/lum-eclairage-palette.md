# LUM : éclairage, ciel et palette unifiés campagne / bataille

Branche `worktree-agent-abf125ec26f3d1ce0`. Source : `out/juge/iter-02-art.md` (constats 3, 4, 5, 8, chantier 3). ADR 0748.

## Plan
1. Bible de palette : `docs/design/palette.md` + `data/fx/palette.json` (+ schéma) ; saison de référence printemps ; test de cohérence avec `atmosphere.json`.
2. Bataille : 6 clés d'heure (dawn, morning, midday, afternoon, dusk, night) avec soleil, étalonnage, brume colorée ; cause des taches du dusk ; ciel plus présent.
3. Campagne : même étalonnage commun (saturation, contraste), voile retiré en météo claire (`campaign_post.json`), brouillard (ADR 0596) intact.
4. Herbe de bataille : macro-variation de teinte pilotée par bruit monde (`ground.wesl`).

## État
- [ ] squelette (ce fichier)
