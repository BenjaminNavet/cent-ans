# FA6 — vue parchemin : ornements de portulan réels (2026-10-02)

Branche `feat/fa-map`, worktree `../gp-fa-map`. Périmètre strict : vue parchemin seulement
(`parchment_decor.gd`, `parchment_overlay.gd`, `parchment_sea.gdshaderinc`, nouveaux fichiers).
Brutes hors dépôt : `~/dev/cent-ans-raw/fa/campaign/` ; captures : `~/dev/cent-ans-raw/fa/map-shots/`
(`ab-<vue>.png` = avant | après).

## Consigne de reprise
> Lot terminé côté agent : lis « Défauts restants » avant toute retouche, puis `git log --oneline -5`.

## État : TERMINÉ (à juger et fusionner par la session principale)
- [x] Outil `tools/cent_ans_tools/parchment_ornaments.py` (estimation locale du vélin, clé d'encre,
      rejet des vagues bleues, masques disque / polygone à trous, `--preview`, `--licences`),
      catalogue `data/map/parchment_ornaments.json`, schéma
      `data/schemas/map_parchment_ornaments.schema.json`, test
      `tools/tests/test_map_parchment_ornaments_schema.py`, sorties
      `game/assets/textures/parchment/` (3 PNG, 0,8 Mo) + `SOURCE.md` écrit par l'outil.
- [x] Rose des vents de l'Atlas catalan (1375) : échantillonnée dans `parchment_sea.gdshaderinc`
      (`pm_rose_tex`, texture par défaut du shader `water.gdshader` posée par `ParchmentDecor` :
      ni `sea.gd` ni `strategic_view.gd` ne sont touchés) ; à plat sur la carte, nord en haut ;
      les rhumbs existants partent du cercle de la rose.
- [x] Navire (nef de la mer des Indes, feuillet 10) et monstre (sirène à deux queues) : sous-couche
      `SeaLayer` de `ParchmentOverlay` (mipmaps), une sirène sur deux en miroir. Les ornements
      peints sont plus grands que les dessins : `ParchmentDecor` leur cherche de l'eau libre
      (monstres) et répartit les navires au plus loin les uns des autres, cœur de carte d'abord.
- [x] `--no-fa-parchment` (après `--`) : ancien dessin par code et anciens emplacements (A/B).
      Catalogue ou textures absents (fixtures du smoke) : même repli.
- [x] Script `game/tests/fa6_parchment_shot.gd` (`--views`, `--list`, `--stats`).
- [x] Crédits (`CREDITS.md`), tests (smoke, cm2, dv ×2, pytest, ruff).

## Essayé puis abandonné
- Uxer de Jaume Ferrer (Atlas, feuillet de l'Atlantique) : esquisse à l'encre pâle, tache grise à
  la taille d'un ornement.
- Noms des vents autour de la rose (tramuntana, levante…) : illisibles et « sales » une fois la
  carte inclinée ; la rose est réduite à son cercle.
- Grand poisson vert (feuillet 10) : aplat sans trait, lu comme une feuille.
- Monstre de Dürer (scène entière, hachures denses) et bronzes de Calzetta : pas de l'encre sur
  vélin, non détourables en ornement ; Bruegel inutile (l'Atlas fournit un navire lisible).
- Grain de vélin (point 4) : le fond actuel n'est pas plat (taches, pâtés, fibres procéduraux) et
  aucun feuillet de l'Atlas n'offre de zone vierge assez grande (rhumbs et texte partout).
- Lignes de rhumb (point 3) : déjà présentes et déjà aux couleurs de l'Atlas (vents à l'encre,
  demi-vents verts, quarts rouges) ; seul leur départ est ajusté au cercle de la rose peinte.

## Coût
Appels de dessin 3D inchangés sur 7 vues (`--stats`) ; 2D : un `CanvasItem` de plus, au plus
2 textures (navire, sirène) à la place des 3 vignettes dessinées ; shader de la mer : un
`textureGrad` dans le carré de chaque rose.

## Défauts restants
- Le navire est une nef de la mer des Indes vue par l'enlumineur majorquin (voiles de nattes) :
  d'époque et de la même main que la rose, mais pas une cogue de la Manche.
- Ornements dressés (billboards) de 70 à 90 px : dans les mers étroites (Manche, Baltique) le
  navire déborde sur la côte située derrière lui.
- Un seul navire et un seul monstre (répétition à l'écran au zoom maximal).
- Rose coupée par les côtes dans les mers étroites (Adriatique, Baltique), comme avant.
- Les 3 vignettes dessinées restent créées au démarrage même si elles ne servent pas.
