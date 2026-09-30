# 0146 — Deux fois plus de régiments par bataille

Date : 2026-09-30

## Contexte
Demande du joueur : doubler le nombre d'unités présentes lors d'une bataille. Le plafond valait 20 régiments par armée (ADR 0128) et 20 / 40 / 80 régiments sur le champ par camp selon le palier d'échelle (ADR 0076).

## Décision
Tous les plafonds de régiments sont doublés ; l'effectif d'un régiment ne change pas.
- `data/rules/armies.json` : `max_units` 20 → 40 (formation, fusion, mercenaires, ralliements).
- `data/rules/battle_scale.json` : régiments sur le champ par camp 40 / 80 / 160 (escarmouche / grande bataille / bataille rangée).
- Les seuils d'effectif des paliers ne bougent pas (escarmouche ≤ 4 000 soldats, grande bataille ≤ 8 000) : le champ reste choisi par l'effectif. Les doubler plaçait 60 contre 60 régiments sur le champ de 1800 m, où l'attaquant gagnait 10 fois sur 10 (test EP9b). `MAX_ON_FIELD` (champ standard, sièges) passe à 40.
- Batailles historiques (Crécy, Poitiers, Azincourt) : 160 sur le champ.
- Bataille personnalisée : 40 unités par camp ; budget par défaut 12 000 (bornes 2 000 – 60 000) pour que le plafond reste atteignable.
- Schémas élargis (armée ≤ 80, carte historique ≤ 200).

## Conséquences
- Le plafond sur le champ n'est presque plus jamais atteint en bataille de campagne (40 régiments de 100 hommes dépassent déjà 4 000 soldats) : les renforts échelonnés ne jouent plus guère qu'en siège ou au-delà de 160 régiments par camp.
- Référence de déploiement CB6 régénérée pour le seul scénario `big` (ses 40 régiments français sont désormais tous sur le champ).
- Coût de simulation et de rendu en hausse ; à mesurer sur machine calme (banc EP1).
- L'IA de campagne forme des armées plus grosses (plafond lu dans les données).
