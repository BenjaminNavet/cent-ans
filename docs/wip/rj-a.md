# RJ-a — formations historiques, reformation progressive, état des boutons d'ordre

Branche `feat/rj-a`, worktree `../gp-rj-a`. Chantier parent : `docs/wip/rj-retours-joueur.md`. ADR 0174.

## Plan
1. Données : `data/rules/unit_formations.json` + `data/schemas/unit_formations.schema.json` (formations, noms historiques, infobulles, catégories permises, géométrie, modificateurs, durée de reformation).
2. Core : `Formation` devient un identifiant de la table de données (sérialisé par sa clé) ; tous les effets lus dans la table ; IA par rôles.
3. Core : reformation progressive (`Unit::reform`), figurines qui marchent de l'ancienne place à la nouvelle, malus en données ; `reforming` + progression dans `get_units`.
4. Pont : `get_formations()` (catalogue + permis par unité).
5. UI : menu de formations (infobulles riches), formation courante surlignée, état on/off de « formation » et « tir à volonté », barre de reformation.
6. Tests cargo + GDScript, ADR 0174, recherche `docs/research/rj-formations.md`.

## État
- squelette (cette note, ADR brouillon).

## Prochaine étape
Données + type `Formation` data-driven dans `sim-battle`.
