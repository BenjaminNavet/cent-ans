# FA6 — vue parchemin : ornements de portulan réels (2026-10-02)

Branche `feat/fa-map`, worktree `../gp-fa-map`. Périmètre strict : vue parchemin seulement
(`parchment_decor.gd`, `parchment_overlay.gd`, `parchment_sea.gdshaderinc`, nouveaux fichiers).
Brutes hors dépôt : `~/dev/cent-ans-raw/fa/campaign/` ; captures : `~/dev/cent-ans-raw/fa/map-shots/`
(`ab-<vue>.png` = avant | après).

## Consigne de reprise
> Lot terminé côté agent : lis « Défauts restants » avant toute retouche, puis `git log --oneline -5`.

## État : TERMINÉ (à juger et fusionner par la session principale)
Retenu après relecture de la session principale : **rose et sirènes réelles, navire dessiné**.
- [x] Choix par genre dans le catalogue : `"use": {"rose": "real", "ship": "drawn", "monster": "real"}`
      (`data/map/parchment_ornaments.json`, lu par `ParchmentDecor`) ; `--no-fa-parchment` force
      tout en dessiné (A/B) ; catalogue ou textures absents (fixtures du smoke) : même repli.
- [x] Outil `tools/cent_ans_tools/parchment_ornaments.py` (estimation locale du vélin, clé d'encre,
      rejet des vagues bleues, masques disque / polygone à trous, lavis sous l'encre, `--preview`,
      `--licences`), schéma `data/schemas/map_parchment_ornaments.schema.json`, test
      `tools/tests/test_map_parchment_ornaments_schema.py`, sorties
      `game/assets/textures/parchment/` (3 PNG, 0,8 Mo) + `SOURCE.md` écrit par l'outil.
- [x] Rose des vents de l'Atlas catalan (1375) : échantillonnée dans `parchment_sea.gdshaderinc`
      (`pm_rose_tex`, texture par défaut du shader `water.gdshader` posée par `ParchmentDecor` :
      ni `sea.gd` ni `strategic_view.gd` ne sont touchés) ; à plat sur la carte, nord en haut ;
      les rhumbs existants partent du cercle de la rose.
- [x] Sirène à deux queues (Atlas, feuillet 12) : sous-couche `SeaLayer` de `ParchmentOverlay`
      (mipmaps), une sur deux en miroir ; placées en eau libre, à l'écart des navires.
- [x] Navire : dessin par code d'origine, emplacements d'origine. L'uxer de Jaume Ferrer
      (`ship_ferrer.png`, trait renforcé, lavis de voile et de coque) reste livré : passer
      `use.ship` à `real` l'affiche (avec la répartition « eau libre » des navires peints).
- [x] Les vignettes dessinées ne sont créées que pour les genres restés en `drawn`.
- [x] Script `game/tests/fa6_parchment_shot.gd` (`--views`, `--list`, `--stats`).
- [x] Crédits (`CREDITS.md`), tests (smoke, cm2, dv ×2, pytest, ruff).

## Essayé puis abandonné
- Navire réel, deux essais : la nef de la mer des Indes (feuillet 10 ; tache gris pâle, et pas un
  navire de nos mers) puis l'uxer de Jaume Ferrer renforcé (encre assombrie, lavis crème sous la
  voile, brun sous la coque) : reconnaissable en pleine mer mais brun sur brun, il se fond dans
  la côte en Manche et se lit moins vite que la nef dessinée (voile blanche, croix rouge).
  Carte Pisane et planches Nordenskiöld (Dulcert 1339) : aucun navire.
- Noms des vents autour de la rose (tramuntana, levante…) : illisibles et « sales » une fois la
  carte inclinée ; la rose est réduite à son cercle.
- Grand poisson vert (feuillet 10) : aplat sans trait, lu comme une feuille.
- Monstre de Dürer (scène entière, hachures denses) et bronzes de Calzetta : pas de l'encre sur
  vélin, non détourables en ornement ; Bruegel non utilisé (postérieur d'un siècle et demi).
- Grain de vélin (point 4) : le fond actuel n'est pas plat (taches, pâtés, fibres procéduraux) et
  aucun feuillet de l'Atlas n'offre de zone vierge assez grande (rhumbs et texte partout).
- Lignes de rhumb (point 3) : déjà présentes et déjà aux couleurs de l'Atlas (vents à l'encre,
  demi-vents verts, quarts rouges) ; seul leur départ est ajusté au cercle de la rose peinte.

## Coût
Appels de dessin 3D inchangés (`--stats`) ; 2D : un `CanvasItem` de plus, une texture (sirène) à
la place de 2 vignettes dessinées ; shader de la mer : un `textureGrad` dans le carré de chaque
rose.

## Défauts restants
- Sirènes dressées (billboards) d'environ 90 px ; un seul monstre (répétition au zoom maximal).
- Rose coupée par les côtes dans les mers étroites (Adriatique, Baltique), comme avant.
- `ship_ferrer.png` (0,2 Mo) est livré sans être affiché par défaut.
