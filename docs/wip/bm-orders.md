# bm-orders : sélection et ordres de bataille (Bevy)

État : fait et prouvé (dump `battle_orders` avec `CENT_ANS_ORDERS_DEMO=1`) : picking CPU, sélection (clic, boîte, Maj, double clic, Ctrl+A…), ordres move/attack/ligne/pivot/file/verrouillage, groupes Ctrl+1..9, raccourcis, aperçu de chemin, arcs de portée, contours, curseur. Tests unitaires : picking, conversions d'ordres, groupes, raccourcis.
Prochaine étape : capacités Alt+1..4 et préréglages de formation de groupe (Godot `battle_formation_picker.gd`), numéros de points de passage des ordres en file, fantômes de formation à l'arrivée, signets de caméra, menu des formations (côté HUD via `IssueBattleCommand`).
Points ouverts : touches physiques (AZERTY), détection de l'interface sous le curseur par heuristique (`HoverMap` + fond opaque), `ground_point` sans test unitaire (pas de constructeur de `Battlefield` vide).
