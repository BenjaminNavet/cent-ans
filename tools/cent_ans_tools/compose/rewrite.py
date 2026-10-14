"""Lot C3 : réécriture des données de jeu pour la carte composée (ADR 0355).

Lit ``data/map/compose/id_map.json`` et ``selection.json`` (C0a) et réécrit en place
``data/provinces``, ``data/settlements``, factions, personnages, titres, événements et
toutes les références (naval, commerce, fleuves, règles...). Idempotent : un
identifiant absent de la table est inchangé.
"""

from __future__ import annotations
