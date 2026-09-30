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
