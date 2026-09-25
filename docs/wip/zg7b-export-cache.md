# ZG7b — cache de relief absent, embarquement, docs, crédits (ADR 0036)

Branche `worktree-agent-afe2f78fb67e7e8a0` (worktree d'agent), partie de `main` 369bc6e7.
Voisin : ZG7a (perf, lit de la Seine, rives de Londres, PathPreview) — ne pas toucher à ses fichiers.

## Plan
1. `ReliefCacheStatus` (game) : détection pyramide absente / partielle (échantillon de tuiles par
   étage, hydro/routes fines) ; avis non bloquant en français, une fois par session, fermable,
   commande de régénération ; journal. Test headless `zg7b_cache_test.gd`.
2. Outils : `cent-ans geo relief-all` (ordre pyramid 1-4 → detail-dem → hydro-fine →
   anchors-fine, reprise), `--check` (liste ce qui manque, code 1). Module
   `geo/relief_cache.py` + tests pytest.
3. Export : `tools/export_macos.sh` suit les liens symboliques de la pyramide, vérifie le cache,
   option d'export sans relief ; `MapPaths` : dossier de relief externe (variable
   `CENT_ANS_RELIEF_DIR`, dossier à côté de l'app). Addendum ADR 0036.
4. Docs : `docs/geo.md` (pipeline complet paliers 1-3), `docs/godot-map.md` (vue d'ensemble ZG),
   `CREDITS.md` (sources MNT et licences).

## État
- 1 fait : `relief_cache_status.gd`, `relief_cache_notice.gd`, crochet d'une ligne dans
  `campaign_map.gd`, `zg7b_cache_test.gd` OK.
- 2 fait : `geo/relief_cache.py`, `cent-ans geo relief-all [--check] [--force] [--workers]`,
  `tests/test_relief_cache.py` (8 tests).
- 3 fait : `cent_ans_tools/export_data.py` + `cent-ans export-data --app … --relief
  bundle|external|none`, appelé par `export_macos.sh` (`CENT_ANS_EXPORT_RELIEF`) ;
  `MapPaths.relief_root_for` ; export debug réel essayé avec une fausse pyramide (E1 partiel) en
  modes bundle et external : relief trouvé, avis PARTIAL journalisé. Tests `test_export_data.py`.

## Prochaine étape
- Addendum ADR 0036, docs/geo.md, docs/godot-map.md, CREDITS.md ; puis smoke + tests ZG.
