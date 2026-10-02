# ADR 0155 — Le rouge est réservé aux ennemis sur la carte de campagne (lot EN)

Date : 2026-10-02. Statut : accepté. Amende l'ADR 0074 (couleur du trait de frontière).

## Contexte

Retour du joueur : « on ne reconnaît pas assez bien les unités, territoires, villes ennemies par
rapport aux neutres ; je joue la France et un voisin a un liseré rouge sur la frontière mais ce
n'est pas un ennemi ». Hors mode Diplomatie (DZ), frontières (ADR 0074), plaques d'armée et villes
ne portaient que la couleur héraldique : un voisin aux armes de gueules se lisait « ennemi », et un
ennemi aux armes d'azur ne se distinguait de rien. La relation n'était visible qu'en changeant de
mode de carte.

## Décision

Dans tous les modes sauf Diplomatie, la couleur dit la relation avec le joueur ; l'identité des
factions reste portée par les écus (villes, plaques) et les étendards.

- Quatre catégories tirées des positions du cœur (`get_faction_stances_for`, rebelles toujours en
  guerre) : nous, ennemi (guerre), ami (allié, vassal ou suzerain), autre (neutre, accord, tension).
- **Frontières** : nous or, ennemis rouge vif, amis vert, autres héraldique assourdie (saturation
  plafonnée : un rouge héraldique en paix devient brique terne). Les royaumes ennemis restent à
  pleine intensité quand le reste de la carte est au repos (TB2) : drapeau dans l'alpha de la
  palette, uniforme `fr1_enemy_focus`.
- **Armées** : bordure de la plaque d'effectif par catégorie, marque ⚔ sur les plaques ennemies
  (lisible sans la couleur), anneau au sol rouge (ennemi) ou vert (ami).
- **Villes** : nom des villes tenues par un ennemi à l'encre rouge. Pas de signe en plus sur
  l'écu (ADR 0151 : un signe par ville).
- Le mode Diplomatie garde sa palette à sept positions et la vue d'une autre faction (DZ).
- Tout est réglable dans `data/map/stance_cues.json` (schéma `stance_cues.schema.json`) ; la
  logique de rendu vit dans `StanceCues` (`game/scripts/map/stance_cues.gd`). Aucune règle de jeu
  n'est touchée.

## Options écartées

- Garder le trait héraldique et ajouter un halo rouge aux ennemis : un voisin de gueules en paix
  reste rouge, le défaut signalé demeure.
- Palette du mode Diplomatie partout (jaune neutre, orange tension, bleu accord) : carte bariolée,
  contraire au calme voulu par TB2, et orange trop proche du rouge.

## Conséquences

- La couleur héraldique des frontières ne se lit plus qu'assourdie ; elle reste entière sur les
  écus, étendards, minicarte et teinte des provinces.
- Une déclaration de guerre ou une paix repeint frontières, plaques et noms au rafraîchissement
  suivant, sans reconstruire les figurines (la catégorie est hors signature PB1).
- Coût : une lecture de texture de plus par pixel de frontière ; une requête de positions par
  rafraîchissement et par calque (frontières, armées, villes).
- Test `game/tests/en_stance_cues_test.gd`, captures `game/tests/en_shot.gd`.
