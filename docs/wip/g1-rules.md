# Lot G1 « Dernières règles inertes » — état

Branche : `worktree-agent-aae49b73351b126ff`.

## Points
1. [x] `recruit_slots` : 2 + 1 (capitale) + effets ; `OrderError::RecruitQueueFull` ; l IA respecte les places libres.
2. [x] `Piety` : traits → `religion::effective_piety` (faveur, hérésie) ; bâtiments → +1 piété/an au souverain par 10 (plafond 3), hiver.
3. [x] `army_armor`/`army_ranged` : `buildings::levy_bonus`, stocké sur l unité (`Unit::levy_armor`/`levy_ranged`, plafond 10), auto-résolution (`side_from_army`) + `battle_setup` (stats).
4. [x] `transfer_province { province, faction?, from? }` (jamais une capitale) : Dauphiné (from Empire), Auray/Guérande (Bretagne rendue à Montfort), Formigny (Normandie anglaise → France). Valdemar IV sans objet (pas de faction Danemark), Trévise pas une province.
5. [ ] Rançon côté joueur : `PayRansom`, `ReleaseCaptive`, IA, pont, fiche personnage.
6. [ ] Alliés dans les assauts de siège.
7. [ ] `docs/status.md`, specs, smoke.

## Prochaine étape
Point 5 (rançons : ReleaseCaptive, pont, fiche).
