# bm-bhud6
Faits : infobulles des pastilles de cartes (`cards::badge_tip` : `mode_icons::tip_of` pour les modes, libellé sinon ; `ui/kit/tooltip.rs` : l'hôte le plus interne gagne, texte vide ignoré) ; bulle du chroniqueur en tête de la colonne gauche, au-dessus du journal, avec bouton « × » (`audio/advisor.rs`, n'interrompt pas la voix : pas de message d'arrêt dans l'audio) ; sceau de cire à bord irrégulier (image CPU, `orders_bar::wax_image`, médaillon réduit à 0,8 du rayon comme Godot).
Déjà conformes (dump `CENT_ANS_NO_DEPLOY=1`) : formations repliées (`battle_orders.formations.collapsed` = true), journal replié (`battle_hud.journal.expanded` = false).
Restes : contrôle à l'œil du sceau et de la bulle ; arrêt de la voix à la fermeture.
