# VN — nuit visuelle (2026-09-30)

Mandat : amélioration visuelle libre, autonomie toute la nuit, 100 captures, fal.ai possible
(pas OpenRouter). Branche `feat/vn`, worktree `../gp-vn`. La carte de campagne (biomes, forêts,
rochers) appartient à HB (`feat/hb`) : VN n'y touche pas.

## Compteur de captures lues
94 / 100

## Constats (état des lieux)
- Bataille, vue moyenne : touffes d'herbe sombres en plaques sur sol brun (effet « détritus »),
  rivière bleu uniforme à bords francs (canal), bande pâle à l'horizon.
- Bataille, gros plan : touffes « salade » vert sombre et touffes presque blanches.
- HUD bataille en 1280×720 : panneau « Formations de groupe » au milieu du champ.

## Lots
- VN1 herbe des batailles (couverture, teinte fondue dans le sol)
- VN2 eau des rivières de bataille
- VN3 horizon des batailles
- VN4 HUD bataille en petite résolution
- suite : écrans d'UI (menu, panneaux), sièges

## Fait (feat/vn)
- VN1 herbe verte vue de haut (contraste lointain atténué, trouées réduites, chaume moins blanc).
- VN2 berges mouillées sombres, boue brune en plaques irrégulières (plus de disques ni de taches
  noires), luisance lointaine plafonnée.
- VN4 HUD bataille : pas de bandeau d'alertes vide, journal à une ligne dans sa zone, alertes
  sous le journal, formations repliées au début du combat.
- VN5 pluie/neige effacées près de l'objectif, neige moins laiteuse, eau suivant l'heure.
- Infobulle des routes commerciales : noms des marchandises.
- VN6 champs de blé/chaume : sol couleur paille, parcelles entières (plus d'ovales).
- VN7 monuments 1:1 : baies, portail, rose (cathédrales), arcades (halles), baies (palais, beffrois).
- Écus des colonies lointaines masqués en vue rasante (plus de rangée d'écus sur l'horizon).
- VN8 bandeau d'ost de campagne : cartes illustrées comme en bataille.
- VN9 12 miniatures d'unités manquantes (fal.ai, 0,96 $, budget.md).
- Brouillard : allégé au-dessus de la nappe basse.
- Bataille personnalisée : miniatures d'unités dans les listes.
- Villes génériques 1:1 : sol en terre/jardins (plus de galette noire), pas de sol loin des maisons.
- Noms des fleuves plafonnés à 40 × la distance caméra (plus d'alignement sur l'horizon).
- Chemins de bataille en terre brune ; miniature de bld_collegiate_church (0,08 $).

## UI 720p (agent, fusionné ; détail docs/wip/vn-ui-720.md)
- Lot 1 : menu principal tient en hauteur, tutoriel s'efface sous une modale, panneau de province
  au-dessus de la mini-carte, bandeau de tour qui passe à la ligne (test `vn_ui_720_test`).
- Lot 2 : chronique bornée, fiche de ville, journal en tête des avis, budget, objectifs, fiche
  personnage (test `vn_ui_720_b_test`).
- Session principale : fiche personnage centrée sous la barre du haut (elle s'ouvrait hors écran,
  y = -1040, en fenêtre réelle).

## Points ouverts (à trancher / plus tard)
- 147 factions sans miniature d'encyclopédie (~12 $ en fal.ai) + bld_collegiate_church : non fait
  (dépense non justifiée pour une seule page d'encyclopédie).
- Tours de siège énormes (rayon 5+fortif m, cœur `siege_layouts.rs`) : règle du cœur, session
  siège en cours sur main → non touché.
- `q6_ui_test` (SimFacade introuvable avec --script) et `fe_ui_test` échouent aussi sur main.

## Intégré dans main
7e68b23e3 (ff-only, 30/09 23:30), dylib reconstruite.

## Lot 3 UI (fusionné, 147affacd)
Diplomatie resserrée (min 1060 px), sommaire du tutoriel défilant, faits notables de l'écran de
résultat dans leur propre défilement (cartes des régiments visibles ; > ~20 régiments/camp : 3e
rangée coupée). Test vn_ui_720_c_test (vues 1138×640 à 1920×1080). ub1_ui_test adapté au journal
replié par défaut (VN4).

## Prochaine étape
Chantier clos. Restent les points ouverts ci-dessus (tours de siège, miniatures de factions ~12 $).

