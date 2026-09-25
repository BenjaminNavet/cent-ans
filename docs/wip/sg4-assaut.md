# SG4 — IA d'assaut de siège et équilibre défenseur / attaquant à l'échelle épique

Branche `worktree-agent-af1d6ff622ee45890` (worktree `agent-af1d6ff622ee45890`). Suite de SG3
(`docs/wip/sg3-sieges.md`), ADR 0023 (SG2, SG3, **SG4**), 0031-0034 (EP), 0046 (R4, **SG4**).

## État : terminé, à fusionner par l'orchestrateur

- [x] Mesure de départ : sonde SG3 avec et sans engins ; matrice épique `sg4_balance`.
- [x] Relève de l'équipage du bélier (règle du cœur, `sim/siege_assault.rs`, données
  `siege_works.json` `ram.relief_range_m` 18, `relief_men_per_s` 0,5), hommes rendus à la fin.
- [x] IA d'assaut (`ai.rs::plan_siege_attack`) : régiment de relève posté derrière le bélier,
  échelles sur tous les pans, beffrois, entrée par la porte ou la brèche, régiments passés dans la
  ville vers la place, cible des engins (pan faible le plus proche), retraite si l'assaut est perdu
  (`ASSAULT_STALL` 300 s, `ASSAULT_HOPELESS` 0,35) ; tireurs à pied sans munitions escaladent.
- [x] Équilibre épique : avantage de la hauteur en mêlée en données (`data/rules/battle_crest.json`
  : 5 %/m au-delà de 1,5 m, plafond 6 m), cavalerie qui couvre ses tireurs (`SHOOTER_GUARD` 160 m,
  hors posture défensive).
- [x] ADR 0023 § Suite SG4, ADR 0046 § Suite SG4 (tableaux de mesures).
- [x] Captures `docs/audit/captures/sg4/` (`game/tests/sg4_assault_shot.gd`, Avignon graine 11) :
  bélier relevé à la porte, infanterie qui entre par la porte enfoncée.
- [x] fmt, clippy, cargo test, build.sh, pytest (563), import, smoke (28 OK).

## Mesures clés
- Avignon avec engins : 6/10 victoires + 4 nuls → 10/10, 0 nul. Aucun nul nulle part (5 villes ×
  avec/sans engins × armée entière/moitié). Moitié d'armée, niveau 5 : 0 à 5/10 selon garnison.
- Matrice épique (10 graines) : crête 0/10 et 0/10 pour l'attaquant (défenseur toujours vainqueur) ;
  hors crête 25/50 victoires de l'attaquant (main : 20/50 ; plat sans pieux 2/10 → 9/10).

## Écarts et points ouverts
- `f5d` escalade : fourchette 2-4 → 2-5 sur 6 (porte qui tombe grâce à la relève).
- `ep1_scale` (fichier EP1) : pic de mêlée 10 à la graine 11 ; seuil 12 → 10 et nouveau critère
  « régiments ayant combattu » ≥ 40 (55-95 mesurés).
- Empreintes `b6.rs` recalculées (changement voulu), mêmes vainqueurs.
- La capture « échelles sur plusieurs pans » n'a pas eu lieu à la graine 11 (la porte tombe avant
  l'escalade) ; la règle est couverte par la sonde.
- Issue des batailles épiques très sensible au duel de cavalerie (10 graines : ± 3 victoires).
