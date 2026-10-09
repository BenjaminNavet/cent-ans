# SC GT3 + GT7 : tests TestCase et découpage du smoke

Branche `sc/smoke`.

- GT3 : 43 tests `*_test.gd` migrés vers `TestCase` (hors `dn_*`). Échecs préexistants (données/dylib) :
  `nt5_cap_engines_test` (plafond d'armée), `r2_relief_bc5_test` (bande BC1).
- GT7 : `smoke.gd` ne garde que `_init` ; les sections sont dans `smoke_{base,map,campaign,ui,battle}.gd`,
  chaînés par héritage (base → carte → campagne → interface → bataille) pour partager l'état.
- Vérifié : smoke.gd complet en headless, 32 "smoke OK", code de sortie 0.
