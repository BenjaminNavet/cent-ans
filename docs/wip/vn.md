# VN — nuit visuelle (2026-09-30)

Mandat : amélioration visuelle libre, autonomie toute la nuit, 100 captures, fal.ai possible
(pas OpenRouter). Branche `feat/vn`, worktree `../gp-vn`. La carte de campagne (biomes, forêts,
rochers) appartient à HB (`feat/hb`) : VN n'y touche pas.

## Compteur de captures lues
39 / 100

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

## Délégué
- Agent VN-UI (worktree ../gp-vn-ui, feat/vn-ui) : menu principal 720p, tutoriel sur modale,
  panneau de province sous la mini-carte, bandeau de tour tronqué.

## Points ouverts (à trancher / plus tard)
- Tours de siège énormes (rayon 5+fortif m, cœur `siege_layouts.rs`) : règle du cœur, session
  siège en cours sur main → non touché.
- Planche P2a : la fiche personnage ne s'ouvre pas (`--stage=skills`), mise en scène à vérifier.
- UI 720p, 2e lot : chronique (texte coupé à droite), fiche de ville (garnison coupée), agents
  (journal au milieu à gauche), budget (colonne « Écart » coupée), objectifs sur le journal,
  bandeau de saison sous le panneau latéral.
- Champs de blé procéduraux en ovales (Crécy, Poitiers) plutôt qu'en parcelles.

## Prochaine étape
Suite des visuels de bataille / 2e lot UI à l'agent.
