# SS — sol « satellite » de la campagne

Spec : `docs/superpowers/specs/2026-09-30-ss-sol-satellite-design.md` ; ADR 0141. Branche `feat/ss`, worktree `../gp-ss` (dylib copiée, `data/map/pyramid` = lien vers le checkout principal, import fait).
Autonomie totale (joueur 30/09). GA3 fusionnée avant démarrage (condition du joueur remplie).

## Diagnostic (captures avant, `game/tests/ss_shot.gd`, Paris 900/300/90)
Terres ouvertes gris-beige pâle en vue moyenne/proche, ombrage du relief + hachures dominants, parcelles invisibles hors `fp_near`, routes = traits crème uniformes.

## Lots
- [ ] SS1 outil `cent-ans geo colormap` (agent, branche `feat/ss-colormap`, worktree `../gp-ss-cm`) : BC1 14336×12288 + mipmaps, `map.json.colormap.bc1`, style YAML + schéma, pytest.
- [ ] SS2 shader (session principale) : chargement `colormap` (ReliefLandcover.load_gpu_copy), couleur de base, parcelles colorées par bloc, relief abaissé, `road_line` en terre, repli ; test headless `ss_colormap_test.gd`.
- [ ] SS3 lacs (agent, branche `feat/ss-lakes`, worktree `../gp-ss-lakes`) : `data/map/lakes.json` (outil) + `lakes_renderer.gd` + test.
- [ ] SS4 fal.ai textures de détail (après captures de SS2).
- [ ] SS5 captures après, banc perf, statut ADR.

## Prochaine étape
SS2a : dosage relief + palette cultures + routes dans `terrain.gdshader` / `road_line.gdshader` (indépendant de SS1).

## SS3 — lacs (branche `feat/ss-lakes`, worktree `../gp-ss-lakes`)
État : outil `cent-ans geo lakes` (`tools/cent_ans_tools/geo/lakes.py`) → `data/map/lakes.json` : 762 nappes (67 nommées), 13 473 sommets ; composantes d'eau intérieure ≥ 30 px, découpées en bassins plats (un niveau chacun, hauteur modale de `heightmap_render.png`, tolérance 3 m) ; ignorées : 87 sous le niveau de la mer, 230 sans bassin plat (vallées reconstituées de retenues, lacs trop fins comme le Loch Ness) ; retenues exclues : `modern_reservoirs.json` + « Reservoir » Natural Earth sauf lacs naturels régulés (liste `NATURAL_REGULATED_LAKES`). Noms : `--natural-earth <main>/tools/geo/raw/natural_earth/ne_10m_lakes/ne_10m_lakes.shp` (absent du worktree). 10 tests pytest.
Rendu : `game/scripts/map/lakes_renderer.gd` (triangulation, débord 0,5 px sous la rive, 12 088 triangles, 15 ms), mode `sheet` de `river_water.gdshader`, branché dans `campaign_map.gd` (masqué sur le parchemin). `terrain.gdshader` non modifié (l'eau peinte reste dessous, nappe à 0,88 d'opacité). Test `ss_lakes_test.gd` OK.
smoke.gd OK. Prochaine étape : jugement visuel par la session principale (pas de capture ici), fusion dans feat/ss par l'orchestrateur.
