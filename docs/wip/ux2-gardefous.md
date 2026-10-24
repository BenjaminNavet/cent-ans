# UX2 : garde-fous et UX (wip)

Branche : worktree-agent-a61d8ee004a6f80db. ADR 0774 (garde-fous), 0775 (focus, avis, aide).

## Fait
- A1 `client/src/shader_check.rs` : compile (wesl, imports Bevy reels du registre cargo) + valide (naga) les 51 .wesl ; mutation `coast_sampler` verifiee rouge. `SHADER_CHECK_DUMP=<dir>` ecrit les WGSL.
- A2 run_bg.sh, smoke.sh, shuffle.sh : echec sur `failed to create shader module`, `no definition in scope`, `Shader compilation error`.
- A3 bench.sh / ab.sh : tools/heavy.sh + `tools/bevy_bench/calm.sh` (charge < 15, aucun autre client), journal `calm: waiting ...`.
- B1 F9 (et non F8, console) engage le focus ; l'aide F1 a une section « Clavier et manette ».
- B2 `FocusDefault` sur le bouton de confirmation des modales (Oui / Continuer).
- B3 HUD nu = une seule portee de focus (toutes les couches), bataille incluse.
- B4 libelles abreges (data/ui/hud_labels.json `short`) sous 1500 px logiques.
- B5 texte d'etat vide des Causes (data/ui/causes.fr.json).
- B6 Diplomatie differee derriere une modale, offre de missions differee derriere une fenetre centrale (+ toast).
- B7 stick gauche = fleches du focus ; limite manette dans l'aide.
- B9 core : un seul message pour capitulation + chute de la place.

## Termine
B8 (faction select 4 -> 0 echecs), B10 (alerte siege_against), ADR 0774/0775, README. Clippy lib+tests propre, tests cibles verts (focus, shader_check, advisor, hud, notify, missions, modal, faction, chronicle ; core m8).

## Ouvert
- Sondes ui_kit.layout de ~1 px sur hud:feed / hud:journal / advisor:band (campagne sans guide) : echelle UI < 1.
- Voix de adv_siege_against non enregistree (sous-titre seul).
- Focus : pas de defilement automatique vers un bouton hors de vue.
