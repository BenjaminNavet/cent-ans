# V3 — atmosphère : lumière, ciels, feu et fumée (lots A1-05, A1-14, A1-13)

Agent V3, 25/09. Rendu seulement (aucune règle touchée). Source : `docs/audit/a1-visuel.md`.

## Plan
1. **A1-05** ciels HDRI + étalonnage (LUT 3D générée) par météo et saison, bataille et campagne.
   - HDRI Poly Haven CC0 2k dans `game/assets/third_party/skies/` (2 de D0 + 7 ajoutés).
   - `tools/cent_ans_tools/hdri_sun.py` : position du soleil dans chaque HDRI (aligne la lumière).
   - Réglages : `data/fx/atmosphere.json` (schéma `data/schemas/fx_atmosphere.schema.json`).
   - Code : `game/scripts/visual/atmosphere_library.gd` (ciels, LUT), `game/shaders/hdri_sky.gdshader`,
     `battle_atmosphere.gd` et `campaign_atmosphere.gd` branchés dessus.
2. **A1-14** qualité de rendu : `game/scripts/visual/render_quality.gd`, réglage `video/quality`
   (Basse, Moyenne, Haute, Ultra) dans Réglages > Affichage ; SSIL, brouillard volumétrique,
   ombres en cascades, glow, SDFGI (Ultra, bataille). `--quality=` en ligne de commande.
3. **A1-13** feu et fumée : flipbooks procéduraux (`tools/cent_ans_tools/fire_flipbooks.py` →
   `game/assets/textures/fx/`), shaders `fire_flame` / `fire_smoke`, braises, lumière vacillante,
   dans `siege_fire_fx.gd`.

## État
- [x] Squelette, HDRI téléchargés, crédits
- [ ] A1-05
- [ ] A1-14
- [ ] A1-13
- [ ] Mesures FPS avant/après, captures `docs/audit/captures/v3/`
- [ ] import + smoke (23 « smoke OK »)

## Prochaine étape
Implémenter A1-05 (atmosphere_library + hdri_sky).
