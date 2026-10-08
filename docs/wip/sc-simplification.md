# SC — simplification de la base de code (2026-10-08)

Demande du joueur : grand chantier de simplification (moins de lignes, structure, commentaires de
fonctions, doc, design patterns, performance ; passage en Rust des portions GDScript coûteuses).
Mandat (réponses du 08/10) : mécaniques **libres** (simplifier/supprimer si le code y gagne, ADR
à chaque fois) ; supprimer outils one-shot et sondes obsolètes, condenser les wip clos, garder
les ADR ; **20 agents Sonnet par vague** ; toucher aussi les fichiers de la session FL et les
modifs non commitées de main ; **push** en fin de chantier. Coût cloud : 0 $.

Worktree `../gp-sc`, branche `feat/sc` (dylib copiée de main, `data/map/pyramid` en lien).
Modifs non commitées de main au départ sauvegardées dans le scratchpad (`main-dirty.patch`).

## Taille de départ (lignes)
Rust 188 773 · GDScript scripts 124 976 + tests 50 382 · shaders 13 269 · Python 107 987 ·
docs 58 940 (508 notes wip, 163 ADR).

## Méthode
- Vague 0 : audit en lecture seule par zone → lots chiffrés (gain de lignes, risque, vérif).
- Vagues suivantes : un agent `cent-ans-mech` par lot, worktree isolé, ≤ 5 lots Rust
  simultanés (disque : une cible cargo par worktree), commits sur branche, fusion ff-only ici.
- Vérification : `cargo test`, `cargo clippy -D warnings`, pytest ; Godot seulement en headless (scripts de test ciblés par les agents, `smoke.gd` complet aux fusions). **Jamais de lancement fenêtré du jeu ni de capture** (demande du joueur 08/10).

## État
- [ ] Vague 0 audit

## Coordination FL (session parallèle, `../gp-fl`)
Gel jusqu'à la fusion de FL : `game/scripts/map/vegetation.gd`, `campaign_map.gd` (_process /
map.misc), déclutter des villes (settlement), `outbuilding_layer.gd`, life reground,
`terrain.gdshader` + includes, `dev/map_bench.gd`. Tout portage Rust d'un point chaud de la carte
est annoncé à FL avant de commencer. Base FL : d30 p50 31 ms, d150 p50 46 ms (map.misc 8-12 ms).
