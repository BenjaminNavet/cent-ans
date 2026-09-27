# 0095 — Contrôles de bataille façon Total War (lots CB)

Date : 2026-09-27. Statut : proposé (réservé, complété au fil des lots).
Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`.

## Contexte

La revue comparative avec Total War: Warhammer 3 a fait des contrôles de bataille la priorité.
Les lots CB touchent tous les entrées de bataille, le pont et les ordres du cœur (donc le rejeu EP13).

## Décision

1. **Nouveau schéma de touches aligné sur WH3.** F = tir à volonté, T = cycle de formation,
   G = garde, R = marche/course, K = escarmouche, M = mêlée, Alt/Option+1-4 = capacités,
   Tab = vue tactique, Maj + clic droit = ordre en file, Ctrl/Cmd+A, Ctrl/Cmd+G, ralenti ×0,5,
   bouton du milieu = rotation/inclinaison (panoramique sur Maj + milieu). Groupes inchangés.
   Entrées extraites dans `game/scripts/battle/battle_input.gd` (CB0).
2. **Deux requêtes du pont en lecture seule** : `preview_path(unit, x, z)` et
   `hover_context(x, z, selected)`. Aucune ne modifie l'état simulé (`state_digest` inchangé).
   `preview_path` et l'ordre réel partagent `plan_route`, qui renvoie la chaîne complète de points
   de passage (ligne droite + gué/pont en rase campagne, A* de siège) ; `route()` n'en garde que le
   premier point. Il n'y a pas de navgrid en bataille : la navgrid R3 reste propre à la campagne.
3. **Ordres de chef (armée) ≠ capacités (unité).** Cri de guerre, rallier, pied à terre, pas de
   quartier restent des ordres de chef ; le pavois devient la capacité des arbalétriers (CB4).

## Écarts à la spec, tranchés dans le plan

- Pas de brouillard de guerre existant : champ cœur `spotted`, utilisé par la vue tactique seulement.
- Pas d'état « blessé » : l'alerte couvre « général tué ou capturé ».
- Journal non typé : flux typé `BattleAlert` ajouté à côté du journal, qui reste inchangé.
- Largeur de formation : `line_files: Option<u32>` sur l'unité, pas de nouveau type de formation.
- Rejeu : nouveaux champs de `Command` en `#[serde(default)]`, variantes additives ;
  `REPLAY_FORMAT` inchangé tant que les anciens rejeux se relisent.

## Conséquences

À compléter lot par lot (fichiers, chiffres, marges EP7/EQ7 avant/après CB2 et CB4).
