# Lot G1 « Dernières règles inertes » — état

Branche : `worktree-agent-aae49b73351b126ff`.

## Points
1. [x] `recruit_slots` : 2 + 1 (capitale) + effets ; `OrderError::RecruitQueueFull` ; l IA respecte les places libres.
2. [x] `Piety` : traits → `religion::effective_piety` (faveur, hérésie) ; bâtiments → +1 piété/an au souverain par 10 (plafond 3), hiver.
3. [x] `army_armor`/`army_ranged` : `buildings::levy_bonus`, stocké sur l unité (`Unit::levy_armor`/`levy_ranged`, plafond 10), auto-résolution (`side_from_army`) + `battle_setup` (stats).
4. [x] `transfer_province { province, faction?, from? }` (jamais une capitale) : Dauphiné (from Empire), Auray/Guérande (Bretagne rendue à Montfort), Formigny (Normandie anglaise → France). Valdemar IV sans objet (pas de faction Danemark), Trévise pas une province.
5. [x] Rançons : `PayRansom` (H6) existait ; ajout `ReleaseCaptive { character, ransom? }` (≤ rançon calculée, payée comptant par la faction du captif, +3 prestige au geôlier) ; IA : H6 paie déjà (souverains/héritiers par échéances) ; pont `get_character` : captor, captor_name, ransom, ransom_terms, ransom_action ; fiche : ligne + bouton.
6. [ ] Alliés dans les assauts de siège.
7. [ ] `docs/status.md`, specs, smoke.

## Prochaine étape
Point 6 (alliés dans les assauts).
