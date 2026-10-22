# Lot B6 — restes de documentation et d'audio (2026-10-22)

Branche : `worktree-agent-a6d34eac55da5276d` (avancée en fast-forward sur `main` 3942a16b3 avant travail).
Suite de `docs/wip/a2-audio-credits.md` et `docs/wip/a3-documentation.md`.

## État
- [x] (1) `docs/budget.md` : synthèse régénérée par `cent-ans budget summary` (136,39 $ : OpenRouter 59,82 $,
      fal.ai 76,57 $). Aucun montant historique modifié, seule la synthèse en tête de fichier change.
- [x] (2) `docs/manuel.md` § 16 : « moteur Bevy (Rust) » à la place de Godot / godot-rust. Phrase sur les
      musiques corrigée (pistes tierces CC BY, non « générées par les outils »).
- [x] (3) `CREDITS.md` : pistes retirées des playlists annotées « non jouée », fichiers conservés sur disque
      (attribution CC BY maintenue tant que les fichiers sont dans le dépôt).
- [~] (4) `data/audio/music.json` + `credits.json` : périmètre RÉDUIT, voir ci-dessous.

## Point ouvert (choix de conception, à trancher)
Retirer Gibbons, Cabezón, Ortiz, Moulinié, Greensleeves, Istikhbar, Hüseyni et Kapsberger de **toutes** les
playlists `campaign_*` casse deux invariants de `tools/tests/test_era_music.py` :
- chaque `campaign_<région>` doit garder **au moins 2 pistes primary d'époque** (england : 1 restante après retrait,
  burgundy : 1, iberia : 1, islamic : 0) ;
- Kevin MacLeod ne peut pas être en `primary` (islamic n'aurait plus que cela).
Il faut donc choisir des remplaçants d'époque. Candidats déjà sur disque et créditables (constat A2) : Agincourt
Carol (non référencé), Dufay « Ave Regina », Dufay « Se la face ay pale », Binchois/Dufay « Adieu m'amour ».
Aucun candidat d'époque pour islamic (Istikhbar 1929 et Hüseyni pré-1939 sont hors période).

Appliqué dans ce lot : retrait des 6 entrées des playlists `campaign`, `campaign_france`, `campaign_italy`
(elles gardent >= 2 primary), gain et crédit orphelins de Kapsberger supprimés (57 → 56 crédits).

## Hors périmètre laissé tel quel (à décider)
- Playlist `war` : Moulinié en primary (seule piste primary).
- Playlist `court` : Gibbons, Cabezón, Moulinié, Ortiz (anachroniques, mentionnées par le constat A2 pour la cour).
- Playlists `campaign_england`, `campaign_burgundy`, `campaign_iberia`, `campaign_islamic` : voir ci-dessus.
- Fichiers orphelins conservés sur disque (Bach, Le Roy, Hume, Mudarra, maqams 1931, Agincourt, Kapsberger) :
  décider s'ils sont supprimés du dépôt (les crédits CC BY suivraient).

## Tests
`uv run --project tools pytest -q -k "schema or music or audio or credits"` : 1805 passés.
