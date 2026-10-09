# WH charsb — personnages top4/5/7/8/10

Branche wh/charsb (worktree ../gp-wh-charsb). ADR 0284.

## État : code et tests faits ; reste la sonde, le passage clippy/tests complets et le rapport
- top7 : `data/rules/trait_triggers.json` + `trait_triggers.rs` (9 déclencheurs, 6 traits neufs), anciens seuils retirés de dynasty.json.
- top8 : `requires_role` (schéma, `SkillRole`), `skills::roles_of`, 12 compétences de spécialité, filtre dans `learnable_skills`/`learn_skill`.
- top4 : `data/rules/loyalty.json` + `loyalty.rs` (dérive saisonnière, défection, bonus de moral), `EventKind::Loyalty`, pastille fiche.
- top5 : actions `assassinate` / `poison` (effet `strike`), cible = général à portée ou gouverneur, souverain au sceau 4.
- top10 : actions `guide_army` (héraut) / `ambush` (espion), rayon `army_reach_km`.
- Tests : `core/crates/sim-campaign/tests/campaign_life/wh_charsb.rs` (9 tests).

## Reste / points ouverts
- L'IA n'utilise pas les nouvelles actions d'agent (joueur seul) ; icônes des 12 compétences (icons_catalog.py) non générées.
- Pas de baisse de loyauté sur rançon refusée / titre à un rival (rien à accrocher), voir ADR 0284.
- Sonde campaign_probe avant/après : voir plus bas.

## Sonde campaign_probe (120 tours, graines 1,2)
Avant (main 2bfb6177b) : seed 1 guerre FR-EN 59 %, révoltes 0, banqueroutes 113, éliminées 20, France 28 prov ; seed 2 : 52 %, révoltes 7, banqueroutes 123, éliminées 17, France 30 prov.
Après : seed 1 : 65 %, révoltes 3, banqueroutes 75, éliminées 22, France 25 prov ; seed 2 : 68 %, révoltes 3, banqueroutes 127, éliminées 18, France 33 prov.
Lecture : écarts dans le bruit d'une simulation chaotique (le flux aléatoire principal est décalé par les nouveaux traits/compétences choisis par l'IA) ; aucune dérive systématique, 0,8 s/tour inchangé.
