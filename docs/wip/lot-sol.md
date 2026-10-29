# Lot SOL (boucle juge)

État : cause du damier trouvée et corrigée ; forêt désaturée ; tests de garde ajoutés ; vérification finale
(clippy, tests complets, capture après) en cours. Décision : ADR 0735.

- Damier rose : sol texturé pas encore chargé (fil de travail, 2-6 s en qualité haute) -> carte de couleur
  cuite brute visible. Voir ADR 0735.
- Forêt : `data/art/campaign_forest_look.json` (saturation 1,3 -> 0,78, balance neutre, variation de
  peuplement x1,7, clairières plus fréquentes) ; défauts de `vegetation/look.rs` alignés (test
  `shipped_file_parses_to_the_defaults`).
- Prochaine étape : capture « après » (England, 60 et 400 images), commit final.
