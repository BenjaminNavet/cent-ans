# AS8 — animations générées depuis les vidéos libres

Demande du joueur (08/10) : « il faut utiliser les videos libres pour generer les animations ».
Règles : ADR 0189 (licences, vidéos hors dépôt), ADR 0187 (sources par famille).
Sources : `docs/research/as-references-video.md`. Vidéos : `~/dev/cent-ans-mocap-src/video/free/<famille>/`
avec un `LICENSE.txt` par vidéo (auteur, URL Commons, licence vérifiée sur la page).

## Lots (worktrees, une branche chacun)
| Lot | Famille | Sources | Sortie | État |
|---|---|---|---|---|
| AS8a | humains (troupes, servants de canon) | Roscheiderhof, joute Eggenburg, Canon firing | clips NT14 cuits, comparés par `-- measure` | lancé |
| AS8b | chevaux (pas, trot, galop, virage) | Muybridge (DP), Horses running in a pasture (CC BY) | angles suivis → `battle_skinned_gaits.py`, recuit | lancé |
| AS8c | bêtes et charrettes de campagne | montbéliardes (CC0), Ploughing, wagons (CC0), moutons, Rama | cadences/amplitudes/phases → `animal_motion.json`, charrettes | FAIT sur `feat/as8c` (mesures dans `animal_motion_measured.json`, courbes de pas en table, cahot en 3 raies, roulis ; Ploughing inutilisable ; `as8c_test` vert ; restent : jugement en jeu AS7, foulée des moutons, rapport de marche et levée non mesurés) |
| AS8d | engins, feu, herbe, drapeaux | Warwick trébuchet, Canon firing, Fire 01-10 (CC0), herbe (DP), drapeau | courbes → `siege_engines.json`, `map_fire_wind.json` ; flipbook flammes depuis CC0 | lancé |
