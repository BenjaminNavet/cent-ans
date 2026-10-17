# 0476 — Le vaincu se replie hors de la province de la bataille

Statut : accepté.

## Contexte
Le test du 17/10 a cru que le vaincu d'une bataille de campagne restait à Paris : la règle de repli (C7a, ADR 0322) le plaçait bien, mais sur la colonie amie la plus proche, Saint-Denis, à quelques kilomètres dans la même province, et sans mot au journal. À l'écran les deux ost restaient côte à côte.

## Décision
- `retreat_target_after` cherche d'abord une colonie amie hors de la province de la bataille (même rayon, mêmes zones de contrôle) ; seulement à défaut une colonie amie de la même province. Le reste (recul avec traînards, débandade) est inchangé.
- Le repli en bon ordre écrit une ligne au journal (« X, battu, se replie sur Y »), comme le recul et la débandade.
- Butin et prisonniers : le design ne les prévoit pas pour la campagne au-delà du général capturé (rançons, ADR 0321 pour le butin de la poursuite en 3D) ; rien n'est ajouté.

## Conséquences
- Un vaincu quitte la province : le vainqueur la tient, comme à Total War.
- Dans une capitale entourée de places de sa propre province, le repli est plus long (une saison de marche possible) ; l'armée est hors de portée immédiate.
