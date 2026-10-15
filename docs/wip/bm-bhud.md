# bm-bhud : HUD de bataille (client Bevy)

État : fait (cartes, alertes, vitesse, comparaison, résultat, infobulles), testé (24+ tests), vérifié en `--script` Battle.
Restes (Godot) : minicarte (battle_minimap.gd), sceau du chef / barre d'ordres (leader_orders_bar.gd, battle_formation_menu.gd), journal, capacités sur les cartes (battle_ability_icons.gd), groupes Ctrl+1..9 (battle_groups.gd), aide F1, état de siège, suites de campagne + carte historique sur l'écran de résultat (BattleAftermath), bouton « Revoir la bataille ».
Points ouverts : `BattleSelection` n'est appliquée par personne tant que le lot des ordres n'est pas fusionné ; l'écran de résultat n'a pas été vu en vrai (bataille trop longue) : couvert par test unitaire de `build_view`.
