# VN — défauts d'interface en 1280x720

Lot 1 (menu, tutoriel vs modale, province vs mini-carte, bandeau) : fait, `game/tests/vn_ui_720_test.gd`.
Lot 2 : chronique (largeur de contenu bornée à la zone SIDE_PANEL), fiche de ville (lignes vides masquées), journal en tête de pile TOASTS et masqué sous objectifs/cour/techs, tableau du budget (libellés à la ligne), cartouche de saison z_index 100, fiche personnage (hauteur transitoire 2720 px ramenée, repli en colonne sous 960 px de haut). Test : `game/tests/vn_ui_720_b_test.gd` (vues 1422x800 et 1280x720).
q6_ui_test et fe_ui_test échouent aussi sur main (pas causés par VN).
Reste : smoke complet.

## Lot 3 (diplomatie, tutoriel, résultat de bataille)
- Diplomatie : colonnes plus étroites (`FACTION_COLUMN_MIN_WIDTH` 230, `DETAIL_COLUMN_MIN_WIDTH` 380, libellés de clauses 70, chance à la ligne) : largeur plancher du panneau 1060 px au lieu de 1240 (tient dans une vue de 1138 px).
- Tutoriel : sommaire dans `toc_scroll` (ScrollContainer), hauteur bornée à la place sous la barre du haut (`_fit_toc_height`).
- Résultat de bataille : faits notables dans `mentions_scroll` (88 px max, défilement propre) ; cartes des régiments visibles à l'ouverture en 1080p (2 rangées de 14 régiments par camp).
- Test : `game/tests/vn_ui_720_c_test.gd` (vues 1422x800, 1280x720, 1138x640 ; résultat en 1920x1080 et 1280x720).
- `ub1_ui_test` : 2 échecs dans `_check_hud` (journal « (×2) » / journal replié), sans rapport avec ce lot (le résultat de bataille passe).
- Reste : smoke complet.
