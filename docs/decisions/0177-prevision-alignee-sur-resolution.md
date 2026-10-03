# 0177 — Prévision de bataille alignée sur la résolution automatique

Date : 2026-10-03 · Lot A6-L1 (constats M1, M2 de l'audit joueur).

## Contexte

La prévision d'avant-bataille (`battle_forecast.rs`) calculait deux puissances avec `side_power` (formule d'avant le lot N1, sans phases, sans cavalerie contre archers, sans moral de rupture) puis une chance de victoire par intégration d'une fortune de ±10 %. La résolution automatique (`battle_auto.rs`) fonctionne par phases (salves, charge, mêlée), ne désigne pas le vainqueur d'après les puissances mais d'après la rupture du moral. Les deux formules divergeaient : 620 hommes contre 540 donnaient « Défaite presque certaine, 3 % » alors que la bataille se gagnait (pertes 25 % contre 35 %). La barre de rapport de forces (part de puissance) et le texte (probabilité) venaient de deux grandeurs différentes.

De plus, la résolution elle-même était presque déterministe : des forces voisines donnaient 0 % ou 100 % selon un détail (la fortune par phase se moyenne sur huit phases).

## Décision

1. **La prévision exécute la résolution.** Elle lance `resolve_with_crossings` sur `forecast_samples` graines fixes (100 par défaut) avec les mêmes camps, profils, contexte (rivière, passage, murailles, terrain), saison, météo tirée, postures (camp retranché, embuscade) et moral de difficulté que la bataille réelle. La chance de victoire est la part de victoires de l'attaquant ; les puissances affichées sont les moyennes des puissances estimées par la résolution. Aucune formule parallèle ne subsiste (`win_chance` est supprimée). Les graines sont fixes : la prévision reste pure et ne touche pas au générateur de la campagne.
2. **Barre et verdict partagent une probabilité** : `attacker_share == attacker_win_chance`. Côté Godot, `pre_battle_dialog.gd` lit déjà ces deux champs et `BattleUiKit.verdict` ne dépend que de la chance ; aucun changement GDScript.
3. **Brouillard de guerre par bataille** : nouvelle constante `battle_fortune` (`data/rules/auto_resolve.json`, 0,75) : les dégâts de chaque camp sont multipliés une fois par bataille par `1 ± battle_fortune`. Avec la seule fortune par phase, des forces voisines donnaient 0 % ou 100 %. Avec 0,75, des effectifs à ±15 % donnent environ 30-64 % (cas symétriques), et 47 % à égalité.
4. Constantes dans `data/` : `battle_fortune`, `forecast_samples` (schéma `auto_resolve_rules.schema.json`).

## Conséquences

- Écart moyen prévision / fréquence de victoire sur 200 graines : environ 5 points (test `a6_forecast_calibration`, armées de 1337 mises à l'échelle) ; forces voisines (±15 %, compositions identiques) : 30-64 % (test `a6_neighbouring_forces`).
- Une valeur de `battle_fortune` de 0,75 rend toutes les batailles automatiques plus aléatoires, y compris à 2 contre 1. C'est un réglage d'équilibrage à valider par le joueur ; abaisser la constante resserre les probabilités sans autre changement de code.
- Coût : 100 résolutions par affichage de prévision (quelques millisecondes).
- Les parties sauvegardées ne sont pas touchées ; les graines des batailles de campagne changent (deux tirages de plus par bataille).
