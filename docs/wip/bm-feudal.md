# bm-feudal

État : panneau féodal et vues `game_api::feudal` faits et testés (branche bm/feudal).
Reste : liste des événements féodaux de fin de tour, guide de 3 étapes (`feudal_tutorial/done`), filtre de carte `feudal_map_lens.gd`, fil d'Ariane de la fiche de province (`get_province_breadcrumb`), entrée de menu, liste « Colonies » (`holdings_controller.gd`).

## campui7 (vague 8)
Fait : « Événements féodaux » (liste de 12, `FeudalUi::recent`, nourrie par `TurnAdvanced` ; `format::is_feudal_event` = port de `is_feudal_event`) et toasts de fin de tour (appel féodal en attente, ou « N événements féodaux vous concernant ») ; guide de 3 étapes (`feudal/guide.rs`, carte à gauche comme le didacticiel, boutons Continuer / Passer l’étape / Plus tard / Passer le tutoriel, `Settings::feudal_tutorial_done` = `feudal_tutorial/done`, démarre après un tour une fois le didacticiel général inactif, `CENT_ANS_FEUDAL_GUIDE=1|off`).
Écart : le filtre de carte « Féodalité » n’existe pas côté Bevy (`feudal_map_lens.gd` non porté) : l’étape 2 est manuelle et son texte ne renvoie plus à un filtre ; pas de cadre pulsant sur la cible (bouton Arbre féodal).
Reste : filtre de carte `feudal_map_lens.gd`, fil d’Ariane de la province, entrée de menu, liste « Colonies ».
