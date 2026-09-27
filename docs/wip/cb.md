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
| CB1 | Formation au glisser, verrouillage de groupe | 3 | **fusionné 09-27** (répartition des largeurs dans le cœur, `group_gap_m` 10 m ; largeur = passage en Ligne ; `match_speed` = allure en terrain ouvert du plus lent ; bornes de rangs en premières valeurs, sans relecture historique ; cadenas = glyphe en code) | `cb1-formation-glisser.md` |
| CB2 | Modes d'unité (+ « Battre en brèche »), icônes d'état, remappage des touches | 4 | **fusionné 09-28** (table unique `battle_hotkeys.gd` ; drapeaux de mode hors empreinte, leurs effets y sont ; IA : garde seulement en attente ; marges EP7/EQ7 inchangées 16/18/14, 15/16) | `cb2-modes.md` |
| CB3 | Ralenti, caméra (rotation/inclinaison), vue tactique, `spotted` | 4 | **fusionné 09-28** (`spotted` = fonction pure, portée `missile_arc.spotter_range_m` 350 m ; assombrissement par calque HUD) | `cb3-camera-vue-tactique.md` |
| CB5 | Alertes typées, colonne, minicarte, cris | 4 | **fusionné 09-28** (parti d'un main ancien : test cœur remis à jour à la fusion ; cris doublés possibles avec `_detect_events`, amortis par cooldown) | `cb5-alertes.md` |
| CB6 | Formations de groupe (6 préréglages, placement proposé) — demande du joueur 09-27 | 4 | **fusionné 09-28** (Ligne de bataille = placement d'avant au bit près ; sélecteur toujours affiché en bataille, à rendre repliable) | `cb6-formations-groupe.md` |
| CB4 | Capacités actives (5 retenues après relecture historique) | 5 | **fusionné 09-28** (pavois = capacité, ordre de chef retiré, règle gardée pour les anciens rejeux ; IA du tir tendu seulement à l'attaque ; marges Crécy 16→18, Poitiers 14→16, Azincourt 18, EQ7 15 ; carte 112 px, bandeau 140 px ; « Pas de quartier » passe de N à B) | `cb4-capacites.md` |

## Coordination
- Icônes : les agents de la vague 4 dessinent des glyphes en code ; la session principale génère toutes les icônes DA5 (curseurs, cadenas, modes, états, alertes, capacités) en une fois après CB4 (≈ 2 $, `docs/budget.md`).
- Relecture historique CB4 et CB6 faite (09-27, `docs/research/cb4-capacites.md`, `cb6-formations.md`) ; décisions de jeu tranchées par la session principale (joueur : pas de question) et inscrites au plan : noms changés, « Battre en brèche » en mode CB2, 6 préréglages CB6, pieux hors CB.
- 09-27 : le joueur accepte les limites de CB-M2 et autorise l'enchaînement des lots sans nouvelle question.
- CB-M3 et CB-M4 lancés en parallèle (fichiers presque disjoints ; conflit attendu seulement dans `get_units`).
- Budget de captures relevé à **10 par lot** par le joueur (09-27), lues par la session principale seulement.
- CV3-2 fusionné dans main (09-27) : CB-M2 n'attend plus que CB-M1.
- CV3-6 (sondes d'équilibrage) : pas de batailles de référence CB2/CB4 en même temps.
- Fusion vague 4 : CB3, puis CB5, puis CB6, puis CB2 (remappage des touches et aide F1 en dernier).
- Icônes (≈ 40, ≈ 2 $) via le pipeline DA5 ; consigner dans `docs/budget.md`.

## Prochaine étape
Tous les lots de code CB sont dans main. Reste : (1) icônes DA5 en une fois — 6 curseurs, cadenas, modes
(`battle_mode_<mode>`), pastilles d'état, 7 alertes, 5 capacités ; retirer `order_pavise` de
`icons_ink.json` ; plafond 3 $, `docs/budget.md` ; remplacer les glyphes dessinés en code ;
(2) ADR 0095 finalisée (addenda CB-M2…CB6, CB4 : filtre de scénario, IA du tir tendu) ;
(3) partie pilote du joueur.
Suites notées : sélecteur de formations repliable ; cris d'alerte doublés possibles ; bornes de rangs CB1
sans relecture historique ; bulle d'aide CB6 qui masque les archers avancés sur un cadrage haut ; Crécy à
18/20 (bande 14-19) après CB4 : peu de marge haute.
