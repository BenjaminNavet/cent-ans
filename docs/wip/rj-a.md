# RJ-a — formations historiques, reformation progressive, état des boutons d'ordre

Branche `feat/rj-a`, worktree `../gp-rj-a`. Chantier parent : `docs/wip/rj-retours-joueur.md`. ADR 0174 (rédigé).

## Fait
- Données `data/rules/unit_formations.json` + schéma `unit_formations_rules.schema.json` (+ test pytest) : 8 formations (ligne « Rangés en bataille », coin, ordre de marche, schiltron, en haie, bataille serrée, herse, en conroi), bloc `reform`. Sources : `docs/research/rj-formations.md`.
- Core : `Formation` = indice de table (`sim-battle/src/formations.rs`), sérialisé par clé ; plus aucune formation nommée dans le code ; `square_factor` de `battle_push.json` → `push_resistance` du schiltron.
- Reformation progressive : `Unit::reform`, `change_formation`, `tick_reform`, poses interpolées homme par homme (`figure_positions`), malus en données, hachée dans `state_digest`. Tests `tests/rj_formations.rs`.
- IA : rôles (`ai_role`), pas de changement pendant une reformation, marche/coin pas repris avant 20 s, coin seulement à 120-250 m et loin des haies ; détour des cavaliers 25 m côté cible (B6). Empreintes `b6.rs` recalculées.
- Pont : `unit_formations()`, `formation_reform_rules()`, champs `get_units` (battle_sim_formation.rs ; une ligne dans battle_sim.rs).
- UI : menu `battle_formation_menu.gd`, infobulle `ib:formation:<clé>`, boutons Formation/Tir à volonté on/off dorés + libellé + barre de reformation (battle_hud.gd), T = cycle (battle_input.gd), carte d'unité, codex, tutoriel, aide.

## Prochaine étape
Suite cargo complète verte → clippy → build GDExtension → smoke.gd + cb2_modes_test + cb0_input_equivalence_test.

## Points ouverts
- L'IA n'emploie pas encore les nouvelles formations.
- Équilibre : tests de batailles d'IA très sensibles (cavalerie) ; valeurs de reformation à éprouver en partie.
