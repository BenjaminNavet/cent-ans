# WIP — audit A2 mécaniques et équilibre

État : sonde hors dépôt (scratchpad `a2probe` : crate à part qui dépend de `core/crates/{ai,sim-campaign,data-model}` par chemin ; `src/main.rs` = parties IA contre IA, sortie JSON par graine ; `src/bin/matrix.rs` = matrice d'auto-résolution). 5/8 graines × 200 tours agrégées ; 4 × 464 tours en cours.
Constats provisoires : milice urbaine = 99,7 % des recrutements IA ; 10 bâtiments sur 29 jamais construits (tous militaires + cathédrale, étain, pressoir) ; carte figée (≈ 13 changements de propriétaire en 50 ans) ; mécontentement moyen 0,5 ; révoltes ≈ 0 ; France a les 45 technologies en 1387 ; chevaliers 4× moins rentables que la milice en auto-résolution, Crécy gagnée par la France 82-100 %.
Prochaine étape : finir les agrégats (464 tours), rédiger `docs/audit/a2-mecaniques.md`.
