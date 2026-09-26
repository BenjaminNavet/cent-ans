# SZ7 — hébergement du relief fin « Cent Ans relief » (ADR 0077)

Branche `worktree-agent-ab262042c73ec57c4` (worktree d'agent), partie de `main`. Lot SZ7 du
chantier `docs/wip/sz-suites-zoom.md`, suite de ZG7b (`docs/wip/zg7b-export-cache.md`).

## Mandat
Outillage seulement : **aucune publication réseau** (ni `gh repo create`, ni `gh release`) —
réservée à l'accord explicite du joueur. Voir ADR 0077.

1. `data/map/relief_hosting.json` + schéma : URL de base versionnée, nom du paquet, version
   courante liée à la cuisson de la pyramide.
2. `cent-ans geo relief-pack [--out DIR]` : archive `.tar` non compressée (zstd testé sur un
   échantillon synthétique, gain nul sur PNG/bin déjà compressés → décision : pas de compression),
   parts < 1,9 Gio, manifeste JSON (version, parts, SHA-256, crédits), streaming, vérif disque.
3. `cent-ans geo relief-fetch [--dest DIR] [--base-url URL] [--from-dir DIR]` : téléchargement
   repris (urllib Range), vérification SHA-256, extraction atomique, puis `relief-all --check`.
4. `ReliefCacheNotice` : propose `relief-fetch` avant `relief-all`.
5. Tests pytest hors réseau (pack + fetch `--from-dir`, reprise, somme fausse, disque insuffisant).
6. Docs (`docs/geo.md`, commandes de publication à la main du joueur) + addendum ADR 0077 si besoin.

## État
- Squelette en cours d'écriture (ce commit) : schéma + données + modules vides + plan de test.

## Prochaine étape
Implémenter `relief_pack.py` (tar streaming + split + manifeste), puis `relief_fetch.py`
(téléchargement/repli `--from-dir`, vérification, extraction atomique), brancher le CLI, mettre
à jour l'avis du jeu et son test, écrire les tests pytest, documenter.
