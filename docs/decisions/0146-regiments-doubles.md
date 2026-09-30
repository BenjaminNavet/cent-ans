# 0146 — Deux fois plus de régiments par bataille

Date : 2026-09-30

## Contexte
Demande du joueur : doubler le nombre d'unités présentes lors d'une bataille. Le plafond valait 20 régiments par armée (ADR 0128) et 20 / 40 / 80 régiments sur le champ par camp selon le palier d'échelle (ADR 0076).

## Décision
Tous les plafonds de régiments sont doublés ; l'effectif d'un régiment ne change pas.
- `data/rules/armies.json` : `max_units` 20 → 40 (formation, fusion, mercenaires, ralliements).
- `data/rules/battle_scale.json` : régiments sur le champ par camp 40 / 80 / 160 (escarmouche / grande bataille / bataille rangée).
- Batailles historiques (Crécy, Poitiers, Azincourt) : 160 sur le champ.
- Bataille personnalisée : 40 unités par camp ; budget par défaut 12 000 (bornes 2 000 – 60 000) pour que le plafond reste atteignable.
- Schémas élargis (armée ≤ 80, carte historique ≤ 200).

## Conséquences
- Une armée pleine (40 × ~100 hommes) bascule en palier « grande bataille » (champ 1800 m) au lieu d'escarmouche.
- Coût de simulation et de rendu en hausse ; à mesurer sur machine calme (banc EP1).
- L'IA de campagne forme des armées plus grosses (plafond lu dans les données).
