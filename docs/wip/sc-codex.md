# SC codex (DT7) : bundle unique du codex

Etat : mesure avant faite ; bundle genere (`cent-ans codex-bundle`, `data/codex_bundle.json`, schema `codex_bundle.schema.json`) ; test pytest de fraicheur.

Mesure (game/tests/codex_load_bench.gd, 5 `CodexStore.reload`, machine chargee par d'autres agents) :
- avant (476 fichiers) : mediane 227,9 ms, min 61,9 ms, max 418 ms ; empreinte contenu 4a6335f2...d3193

Prochaine etape : `CodexStore.reload` lit le bundle, comparer l'empreinte, retirer la lecture fichier par fichier.
Seul lecteur du codex : game/scripts/codex/codex_store.gd (encyclopedia.gd n'est pas touche).
