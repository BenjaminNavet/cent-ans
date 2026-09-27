# CB — Contrôles de bataille façon Total War (orchestration)

Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`. ADR réservée : `docs/decisions/0095-controles-bataille-tw.md`.

## Lots
| Lot | Contenu | Vague | État | Note wip |
|---|---|---|---|---|
| CB0 | Extraction des entrées (`battle_input.gd`) + sélection rapide | 1 | **fusionné 09-27** (+ correctif caméra : Ctrl/Cmd+lettre ne bouge plus la vue) | `cb0-entrees.md` |
| CB-M1 | Contours de formation (décales), anneau jaune supprimé | 2 | **fusionné 09-27** (trait 1 m, émission sur fond noir ; lisibilité à juger en jeu) | |
| CB-M2 | `preview_path`, `hover_context`, trajets, curseurs | 2 | **fusionné 09-27** (aperçu = ordre exact en ligne droite/pont, ≤ 3 m au gué, ≤ 13 m en siège ; aperçu au clic droit maintenu) | `cb-m2-trajet-curseur.md` |
| CB-M3 | Ordres en file (Maj + clic droit) | 2 | **en cours** (agent, `feat/cb-m3-queue`) | `cb-m3-file-ordres.md` |
| CB-M4 | Portée au sol, comparaison au survol | 2 | **en cours** en parallèle de CB-M3 (agent, `feat/cb-m4-range-compare`) | `cb-m4-portee-comparaison.md` |
| CB1 | Formation au glisser, verrouillage de groupe | 3 | attente CB-M | |
| CB2 | Modes d'unité, icônes d'état, remappage des touches | 4 | attente CB1 | |
| CB3 | Ralenti, caméra (rotation/inclinaison), vue tactique, `spotted` | 4 | attente CB1 | |
| CB5 | Alertes typées, colonne, minicarte, cris | 4 | attente CB1 | |
| CB4 | Capacités actives (relecture historique d'abord) | 5 | attente CB2 | |

## Coordination
- 09-27 : le joueur accepte les limites de CB-M2 et autorise l'enchaînement des lots sans nouvelle question.
- CB-M3 et CB-M4 lancés en parallèle (fichiers presque disjoints ; conflit attendu seulement dans `get_units`).
- Budget de captures relevé à **10 par lot** par le joueur (09-27), lues par la session principale seulement.
- CV3-2 fusionné dans main (09-27) : CB-M2 n'attend plus que CB-M1.
- CV3-6 (sondes d'équilibrage) : pas de batailles de référence CB2/CB4 en même temps.
- Fusion vague 4 : CB3, puis CB5, puis CB2 (remappage des touches et aide F1 en dernier).
- Icônes (≈ 40, ≈ 2 $) via le pipeline DA5 ; consigner dans `docs/budget.md`.

## Prochaine étape
CB-M3 (ordres en file, Maj + clic droit). S'appuyer sur `BattlePathPreview.update_orders` (trajets
des ordres en cours, masqués pendant l'aperçu en direct) pour afficher la file. Pièges : décale Godot 4 =
l'émission ignore l'alpha de l'albédo (fond d'émission noir) ; fenêtre headless 64×64 (forcer
`root.size`) ; capture non headless = Retina 2992 px, réduire à 1600 avant commit ; nouvelles
classes GDScript = `godot --import` avant les tests. Sondes texte : `cbm_outline_shot.gd --probe`,
`cbm2_path_shot.gd --probe`. Curseurs : 6 PNG 32 px à produire (DA5) dans `game/assets/ui/cursors/`,
substituts en code d'ici là. Comparaison calculée par `hover_context`, panneau = CB-M4.
