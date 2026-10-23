# Terrain de campagne en vue rapprochée (lot TERRAIN)

Mandat 23/10 : constats 1, 7, 11, 16 de `out/juge/iter-02-art.md` (sol gris-vert sans matière, forêts en points/boules, hameaux cubes sombres, routes en fils droits). Contrainte dure : aucune régression fps (camp60, ADR 0740-0747, limite de 16 échantillonneurs Metal).
Branche `worktree-agent-affced78d9fe354dc` (base main c6aa7812e+). ADR prévu : 0758.

## Constat de départ
Le sol a déjà tout le mécanisme (45 couches régionales, parcelles/lanières `ground_detail.wesl` + `data/fx/ground_detail.json`, canopée `data/art/campaign_forest_look.json`, cache de sol par nœud ADR 0742). Le mal vient surtout du réglage (contraste, teintes) : le travail est donc d'abord de données + retouches ciblées de shader, sans nouveau sampler.

## État (arrêt demandé par l'utilisateur, 23/10)
Fait (committé, aucun nouveau sampler, aucun coût GPU ajouté) :
- Données `data/fx/ground_detail.json` : dérive de teinte macro réduite (hue 1,0 -> 0,3, plaques rougeâtres), parcelles plus contrastées et saturées, lanières/balks/mouchetures plus fortes, teintes de stade (vert / or / brun ocre). Capture après : `out/t/after_60.png`, `out/t/after_35.png` (parcellaire lisible ; avant : `out/juge/shots/i2/c_loire.png` du dépôt principal).
- `data/art/campaign_forest_look.json` : canopée plus claire (value 0,72 -> 0,88, saturation 1,2, balance olive, variété par arbre/bosquet accrue).
- Routes : `water/ribbon.rs::wander` (lissage de l'escalier de la grille + méandre latéral, extrémités fixes, test unitaire écrit) appliqué aux rubans proches et aux lignes moyennes (`water/mod.rs`) ; `road.wesl` : largeur et usure variables, centre enherbé par endroits, bords fondus.
Non fait : hameaux (toits variés, cours, haies : les teintes viennent des textures de l'atlas, `soot`/`tint` à régler), ADR 0758, mesures fps avant/après (machine à charge 100+), test `wander` non exécuté (le filtre a donné 0 test : le dernier changement de `mod.rs`/`ribbon.rs` n'est PAS compilé ni vérifié ; lancer `cargo test --features dev wander` puis clippy).
Captures utilisées : 2 sur 10. Les captures se sont faites avec une saison non forcée (printemps).
## Prochaine étape
Compiler/tester le dernier changement de routes ; capturer d = 35 (routes) ; hameaux ; ADR ; banc `ab.sh` quand uptime < 20.
