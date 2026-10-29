# Lot SOL (boucle juge)

État : cause du damier trouvée et corrigée ; forêt désaturée ; tests de garde ajoutés ; clippy propre, tests du lot verts ;
deux échecs préexistants sans rapport (`provinces::data::loads_all_provinces_and_raster` 215/213, `menu::historical` liste des batailles). Décision : ADR 0735.

- Damier rose : sol texturé pas encore chargé (fil de travail, 2-6 s en qualité haute) -> carte de couleur
  cuite brute visible. Voir ADR 0735.
- Forêt : `data/art/campaign_forest_look.json` (saturation 1,3 -> 0,78, balance neutre, variation de
  peuplement x1,7, clairières plus fréquentes) ; défauts de `vegetation/look.rs` alignés (test
  `shipped_file_parses_to_the_defaults`).
- Captures : out/before_* (avant), out/after_interim_crop.png et out/after_final_s.png (après), non versionnées.
