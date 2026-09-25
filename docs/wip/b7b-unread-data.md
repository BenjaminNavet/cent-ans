# B7b — données jamais lues par le code

Branche : `b7b-unread-data` (partie de main 04657c09). Contexte : `docs/wip/bulles-partout.md` (B2, vague 3).

## Décisions
1. Piété des édits (Paix de Dieu +2, Carême strict +4) : chiffre annuel versé au souverain chaque hiver, meilleure province seulement (comme la table H3, pas de cumul par province). `edicts::yearly_edict_piety`, appelé par `dynasty::resolve_court_prestige`.
2. `construction_speed` (trait Bâtisseur, compétences Bâtisseur et Urbaniste, et bâtiments/édits éventuels) : durée = arrondi(base × 100 / (100 + %)), au moins 1 tour ; % = bâtiments + édit de la colonie + le meilleur du gouverneur ou du souverain.
3. `recruit_time_turns` : file de recrutement à délai (1 = comportement actuel, l'unité rejoint la garnison en fin de tour) ; les places de recrutement restent « par tour » (seules les recrues commandées ce tour-ci les occupent).
4. `vision_army_km` / `vision_settlement_km` : déjà branchés par le lot M5a (branche `merge-m5a`, en attente de fusion) ; rien à faire ici pour éviter un conflit.

## État
- [x] 1 piété des édits (`edicts::yearly_edict_piety`, test `edict_piety_reaches_the_ruler_each_winter`)
- [x] 2 vitesse de construction (`CampaignState::construction_speed_percent`, `build_time`, plafond `MAX_CONSTRUCTION_SPEED_PERCENT` 100 %)
- [x] 3 délai de recrutement (`QueuedRecruit`, sauvegardes anciennes lues ; IA : l'entretien des recrues en formation compte dans son budget ; pont `recruit_queue_turns`, panneau de colonie « (n tours) »)
- [x] codex : `cdx_jeu_edits`, `cdx_jeu_construction`, `cdx_jeu_recrutement`
- [ ] contrôles complets (cargo test, build.sh, smoke, pytest)

## Prochaine étape
Contrôles complets, puis rapport.
