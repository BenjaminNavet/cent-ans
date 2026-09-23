# H7 — événements historiques manquants (audit § 7)

Générateur : `scratchpad/gen_h7.py` (hors dépôt) ; les JSON sont dans `data/events/`.

## État

- [x] Lot 1 (1337-1340) : cadzand, artevelde, siege_de_dunbar, sac_de_southampton, vicariat_imperial, roi_de_france_a_gand, salado, treve_d_esplechin
- [x] Lot 2 (1341-1357) : mort_jean_iii, arret_de_conflans, morlaix, auberoche, nevilles_cross, traite_de_berwick, ordre_de_la_jarretiere, winchelsea, combat_des_trente, ordre_de_l_etoile
- [ ] Lot 3 (1364-1450) : cocherel, auray, najera, verneuil, pragmatique_sanction, treve_de_tours, prise_de_fougeres, formigny
- [x] Personnages : chr_gautier_de_mauny, chr_william_montagu, chr_agnes_randolph
- [ ] `data/codex/_event_links.md`
- [ ] Tests (cargo test, build.sh, import, smoke)

Grand Schisme : NON créé (déjà traité par `religion.rs` : bascule automatique + offre d'obédience au joueur).

## Prochaine étape

Lot 3, puis _event_links.md et tests complets.
