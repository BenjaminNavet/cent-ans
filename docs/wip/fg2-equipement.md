# FG2 — Équipement fin des figurines de bataille

Branche : `feat/fg2-equipment` (worktree agent). Plan : `docs/wip/fg-figurines-fines.md`.
Précédents : `docs/wip/fg1-corps.md`, `docs/wip/fg0-prototype.md`. main (FG4) fusionné dans la
branche le 26/09.

## Code
- `tools/blender_scripts/battle_fine_gear.py` : registre `GEAR` (constructeur fin par nom
  d'objet V2, repli sur V2 sinon), aides (révolution, balayage, rivets, ourlets `hem`,
  fentes `cut_slit`, UV `unwrap`), casques, armures (plates des membres, jaque, brigandine),
  faces cachées (`cull_hidden`).
- `tools/blender_scripts/battle_fine_weapons.py` : armes et écus (même placement que V2).
- `battle_fine_figures.py` : branchement, harnois par époque, ourlets, faces cachées
  (`hide_covered`), rôles de budget (`GEAR_ROLES`), plafond (`fit_budget`).
- `battle_fine_equipment.py` : jupe FG0 à douze plis (`_skirt(folds=)`).
- `battle_fine_rig.py` : arc long, torse à -75° (FG1 -60°) et épaule d'arc poussée de 20°.

## État
- [x] Squelette, branchement
- [x] Casques : bassinet pointu à camail, à visière museau de chien, ouvert ; cervelière
  (`infantry_5`, `infantry_8` au lieu du bassinet ouvert) ; chapel de fer ; heaume ; salade
  avec ou sans bavière ; chapeau de feutre
- [x] Armures : haubert et camail plus clairs avec ourlet, plates des membres (début : spalières
  3 lames, cubitières, canons d'avant-bras, genouillères, grèves ; tardif : + garde-bras,
  cuissots, 4e lame, gantelets) sur `infantry_0`, `cavalry_0`, `standard_0/1` (début) et
  `infantry_7`, `cavalry_3` (tardif) ; jaque matelassé modelé ; brigandine rivetée ; surcots
  et jupes à plis, fentes de monte, ourlets ; tabards avec ourlet
- [x] Armes et écus : épée XVI, pique à attelles, lance (épieu), javeline, vouge, fourche,
  goedendag, coustille, hache d'armes, lance de guerre à rondelle et pennon fourché,
  couleuvrine, arbalète (arbrier, arc d'acier, étrier, noix, détente), arc long (section,
  poignée, cornes), flèche empennée, carquois ; écu (FG0), rondache, bouclier, targe cloutée,
  adarga, pavois à arête
- [x] Arc long : tirage 0,55 -> ~0,59 m ; flèche visible raccourcie (0,62 m + pointe)
- [x] UV par pièce (projection intelligente hors faces peintes), faces cachées, plafonds
- [ ] Recuisson `rigs` + `figures` (28) + `check`, captures, mesures

## Prochaine étape
`battle_fine.py -- rigs`, puis `figures` (28), `check` ; captures en jeu ; bench.
