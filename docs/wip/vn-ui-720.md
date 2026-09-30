# VN — défauts d'interface en 1280x720

Etat : les 4 points sont corrigés et couverts par `game/tests/vn_ui_720_test.gd` (SubViewport 1422x800 = 1280x720 à l'échelle 0,9 ; headless garde une fenêtre 64x64).
1 menu : paliers de compaction `StartMenu._fit_column`. 2 tutoriel : `TutorialOverlay` s'efface sous une modale (`modal_ok` pour technologies/cour). 3 province : `minimum_size_changed` relance l'ajustement des onglets, garnison en retour à la ligne. 4 bandeau : textes en retour à la ligne.
Reste : lancer les tests existants touchés + smoke.
