# B7b — données jamais lues par le code

Branche : `b7b-unread-data`, rebasée sur main (dc2360c6). Contexte : `docs/wip/bulles-partout.md` (B2, vague 3).

## Décisions
1. Piété des édits (Paix de Dieu +2, Carême strict +4) : chiffre annuel versé au souverain chaque hiver, meilleure province tenue entière seulement (comme la table H3, pas de cumul par province), en plus de la piété des bâtiments (plafonnée à 3 à part). `edicts::yearly_edict_piety`, appelé par `dynasty::resolve_court_prestige`.
2. `construction_speed` (trait Bâtisseur +15 %, compétences Bâtisseur +20 % et Urbaniste +15 %, et bâtiments/édits éventuels, aucun aujourd'hui) : durée = arrondi(base × 100 / (100 + %)), au moins 1 tour, fixée à la commande ; % = bâtiments + édit de la colonie + le meilleur du gouverneur de la province ou du souverain (pas de cumul des deux) ; plafond `MAX_CONSTRUCTION_SPEED_PERCENT` = 100 %. `CampaignState::construction_speed_percent`, `build_time` ; `BuildOption::turns` en tient compte (UI inchangée).
3. `recruit_time_turns` : le design M2 dit « l'unité apparaît en garnison au tour suivant » sans l'exclure, et aucune décision contraire n'est documentée ; la donnée est affichée (« Levée : n tours »), on la branche. File à délai (`QueuedRecruit { unit_type, turns_left, ordered_turn }`) ; 1 tour = comportement actuel (fin du tour de la commande). Les places de recrutement restent « par tour » (message « n par tour » déjà affiché) : seules les recrues commandées ce tour-ci les occupent. Pas d'entretien pendant la formation. Anciennes sauvegardes (ids nus) lues comme recrues d'un tour. IA : l'entretien des recrues en formation compte dans son budget d'armée (sinon elle surrecruterait). Pont : `recruit_queue_turns` ; panneau de colonie : « (n tours) » après les recrues longues.
4. `vision_army_km` / `vision_settlement_km` : branchés par le lot M5a (branche `merge-m5a`, pas encore dans main) ; rien fait ici pour éviter un conflit.

## État
- [x] 1, 2, 3 + tests (`edicts.rs` : `edict_piety_reaches_the_ruler_each_winter` ; `tests/b7b_unread_data.rs` : 5 tests)
- [x] codex : `cdx_jeu_edits`, `cdx_jeu_construction`, `cdx_jeu_recrutement`
- [x] contrôles : fmt, clippy, cargo test (96 binaires verts), build.sh, import, smoke OK, c5_settlements_ui_test OK

## Prochaine étape
Fusion par l'orchestrateur (après M5a si possible).
