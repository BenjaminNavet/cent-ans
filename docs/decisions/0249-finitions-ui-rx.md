# 0249 — Finitions d'interface RX (uifin)

## Contexte
La critique UI/UX de la revue RX a relevé dix défauts de finition (étiquettes tronquées, écran de résultat, cartes de faction vides, apostrophes, chiffres elzéviriens, replis d'identifiants).

## Décision
- Cartes de choix de faction : chiffres clés comparables (provinces, trésor, revenu prévu, armées, vassaux, objectifs). Ils viennent du cœur : `GameDataStore.get_feudal_start_sheets` ajoute `treasury`, `projected_income` et `armies_count` à chaque fiche (état de printemps 1337). Aucune règle côté Godot.
- Chiffres alignés (`lnum`) activés sur les quatre variations Garamond du thème parchemin. La police de titre (IM Fell) n'a pas ce jeu de chiffres : les compteurs doivent utiliser les variations Garamond (Body/Caption).
- Apostrophes : ’ typographique dans les textes visibles. `tools/typo_apostrophes.py` convertit les valeurs de clés d'affichage des JSON de `data/` (jamais identifiants, alias, sources, blasons, invites de génération, voix dont le texte clé le cache audio) et les littéraux entre guillemets doubles de `game/scripts` et `game/tests` ; `--check` échoue s'il en reste. Le paquet du codex est régénéré.
- Replis d'identifiants : test `game/tests/rx_fallback_ids_test.gd` (chaque définition affichée a un `name.display`, aucun texte de `data/ui` n'est un id brut).
- Garnison en lecture seule : lignes identiques regroupées (« 2 × … ») ; les cases à cocher restent une par unité (indices envoyés au cœur).

## Conséquences
- Un nouveau texte visible avec une apostrophe droite se corrige en relançant le script.
- Le panneau de colonie reste ancré dans la zone `SIDE_PANEL` (réduction ou déplacement : non traité, choix de conception ouvert).
- Exclusions du script : les clés `reason` et `reason_fr` (identités comparées au code, p. ex. `PARLEY_REASON`) et les chaînes Rust de `core/` ne sont pas converties ; le codex rend la correspondance des alias insensible à l'apostrophe (`codex_store.gd`, `_plain`).
- Les tests Rust et GDScript qui comparent un texte affiché à une donnée suivent la donnée (’).
