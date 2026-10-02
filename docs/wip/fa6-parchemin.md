# FA6 — vue parchemin : ornements de portulan réels (2026-10-02)

Branche `feat/fa-map`, worktree `../gp-fa-map`. Périmètre strict : vue parchemin seulement
(`parchment_decor.gd`, `parchment_overlay.gd`, `parchment_*.gdshaderinc`, nouveaux fichiers).
Brutes hors dépôt : `~/dev/cent-ans-raw/fa/campaign/` ; captures : `~/dev/cent-ans-raw/fa/map-shots/`.

## Consigne de reprise
> Lis ce fichier et `git log --oneline -8`, puis continue à « Prochaine étape ».

## État
- [x] Outil `tools/cent_ans_tools/parchment_ornaments.py` (détourage : estimation locale du vélin,
      clé d'encre, rejet des vagues bleues, masques disque / polygone / trous), catalogue
      `data/map/parchment_ornaments.json`, sorties `game/assets/textures/parchment/` + `SOURCE.md`.
- [x] Rose des vents de l'Atlas catalan (1375) : échantillonnée dans `parchment_sea.gdshaderinc`
      (`pm_rose_tex`, texture par défaut du shader de la mer posée par `ParchmentDecor`, sans
      toucher à `sea.gd` ni à `strategic_view.gd`) ; les rhumbs partent du cercle de la rose.
- [x] Navire (nef de la mer des Indes, feuillet 10) et monstres (sirène, grand poisson) : sous-couche
      `SeaLayer` de `ParchmentOverlay` (mipmaps), emplacements de `ParchmentDecor` avec eau libre
      exigée autour des ornements peints.
- [x] `--no-fa-parchment` : ancien dessin par code (A/B). Script `game/tests/fa6_parchment_shot.gd`.
- [ ] Schéma JSON + test pytest, crédits, tests Godot, planches avant/après finales.

## Essayé puis abandonné
- Uxer de Jaume Ferrer (Atlas, feuillet de l'Atlantique) : esquisse à l'encre pâle, illisible à la
  taille d'un ornement (tache grise) → retiré.
- Noms des vents autour de la rose : illisibles et « sales » une fois la carte inclinée → rose
  réduite à son cercle.

## Prochaine étape
Schéma + pytest, juger les captures (Manche, Gascogne, Méditerranée), grain de vélin (point 4) à
évaluer, crédits, tests.
