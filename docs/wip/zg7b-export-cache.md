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
- Squelette (ce fichier).

## Prochaine étape
- Implémenter 1 et 2.
