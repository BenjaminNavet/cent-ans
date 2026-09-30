# VN — défauts d'interface en 1280x720

Lot 1 (menu, tutoriel vs modale, province vs mini-carte, bandeau) : fait, `game/tests/vn_ui_720_test.gd`.
Lot 2 : chronique (largeur de contenu bornée à la zone SIDE_PANEL), fiche de ville (lignes vides masquées), journal en tête de pile TOASTS et masqué sous objectifs/cour/techs, tableau du budget (libellés à la ligne), cartouche de saison z_index 100, fiche personnage (hauteur transitoire 2720 px ramenée, repli en colonne sous 960 px de haut). Test : `game/tests/vn_ui_720_b_test.gd` (vues 1422x800 et 1280x720).
q6_ui_test et fe_ui_test échouent aussi sur main (pas causés par VN).
Reste : smoke complet.
