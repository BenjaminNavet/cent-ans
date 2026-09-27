# CB — Contrôles de bataille façon Total War (orchestration)

Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`. ADR réservée : `docs/decisions/0095-controles-bataille-tw.md`.

## Lots
| Lot | Contenu | Vague | État | Note wip |
|---|---|---|---|---|
| CB0 | Extraction des entrées (`battle_input.gd`) + sélection rapide | 1 | **fusionné 09-27** (+ correctif caméra : Ctrl/Cmd+lettre ne bouge plus la vue) | `cb0-entrees.md` |
| CB-M1 | Contours de formation (décales), anneau jaune supprimé | 2 | **fusionné 09-27** (trait 1 m, émission sur fond noir ; lisibilité à juger en jeu) | |
| CB-M2 | `preview_path`, `hover_context`, trajets, curseurs | 2 | **fusionné 09-27** (aperçu = ordre exact en ligne droite/pont, ≤ 3 m au gué, ≤ 13 m en siège ; aperçu au clic droit maintenu) | `cb-m2-trajet-curseur.md` |
| CB-M3 | Ordres en file (Maj + clic droit) | 2 | **fusionné 09-27** (borne 8 dans `battle_queue.json` ; `preview_path_from` ajouté plutôt que changer `preview_path` ; attaque en file close sur cible en fuite seulement si une file attend ; ordre suivant après rupture du contact) | `cb-m3-file-ordres.md` |
| CB-M4 | Portée au sol, comparaison au survol | 2 | **fusionné 09-27** (fusionné avant CB-M3 ; demi-angle de tir = dessin seulement, 60° en données ; panneau posé au-dessus des « Ordres du chef ») | `cb-m4-portee-comparaison.md` |
| CB1 | Formation au glisser, verrouillage de groupe | 3 | **en cours** (agent, `feat/cb1-drag-formation`) | `cb1-formation-glisser.md` |
| CB2 | Modes d'unité, icônes d'état, remappage des touches | 4 | attente CB1 | |
| CB3 | Ralenti, caméra (rotation/inclinaison), vue tactique, `spotted` | 4 | attente CB1 | |
| CB5 | Alertes typées, colonne, minicarte, cris | 4 | attente CB1 | |
| CB6 | Formations de groupe (attaque/défense, placement proposé) — demande du joueur 09-27 | 4 | attente CB1 | |
| CB4 | Capacités actives (relecture historique d'abord) | 5 | attente CB2 (relecture historique **faite**) | |

## Coordination
- Relecture historique CB4 et CB6 faite (09-27, `docs/research/cb4-capacites.md`, `cb6-formations.md`) ; décisions de jeu tranchées par la session principale (joueur : pas de question) et inscrites au plan : noms changés, « Battre en brèche » en mode CB2, 6 préréglages CB6, pieux hors CB.
- 09-27 : le joueur accepte les limites de CB-M2 et autorise l'enchaînement des lots sans nouvelle question.
- CB-M3 et CB-M4 lancés en parallèle (fichiers presque disjoints ; conflit attendu seulement dans `get_units`).
- Budget de captures relevé à **10 par lot** par le joueur (09-27), lues par la session principale seulement.
- CV3-2 fusionné dans main (09-27) : CB-M2 n'attend plus que CB-M1.
- CV3-6 (sondes d'équilibrage) : pas de batailles de référence CB2/CB4 en même temps.
- Fusion vague 4 : CB3, puis CB5, puis CB6, puis CB2 (remappage des touches et aide F1 en dernier).
- Icônes (≈ 40, ≈ 2 $) via le pipeline DA5 ; consigner dans `docs/budget.md`.

## Prochaine étape
CB1 (formation au glisser, verrouillage de groupe, `line_files`, `match_speed`, `group_tag`), puis vague 4
(CB2, CB3, CB5, CB6). Acquis à réutiliser : `preview_group`/`preview_group_queued` (fantômes de groupe),
`BattlePathPreview.update_orders` (file numérotée, masquée pendant l'aperçu), Maj + glisser-droit envoie
déjà `queue: true` (CB1 y ajoute la largeur). ADR 0095 : addenda CB-M2 (invariant aperçu), CB-M3 (fichier
`battle_queue.json`, fin d'attaque sur cible en fuite avec file seulement), CB-M4 (demi-angle de tir =
dessin seulement) à écrire à la finalisation. Pièges : les scripts de capture qui passent par
`scene.issue` contournent `_path_allowed` — poser leurs points sur la terre ferme (sonde `legs=`) ;
panneaux bas-gauche : se poser au-dessus des « Ordres du chef » (`compare_panel.above`).
