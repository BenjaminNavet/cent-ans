# AS8 — animations générées depuis les vidéos libres

Demande du joueur (08/10) : « il faut utiliser les videos libres pour generer les animations ».
Règles : ADR 0189 (licences, vidéos hors dépôt), ADR 0187 (sources par famille).
Sources : `docs/research/as-references-video.md`. Vidéos : `~/dev/cent-ans-mocap-src/video/free/<famille>/`
avec un `LICENSE.txt` par vidéo (auteur, URL Commons, licence vérifiée sur la page).

## Lots (worktrees, une branche chacun)
| Lot | Famille | Sources | Sortie | État |
|---|---|---|---|---|
| AS8a | humains (troupes, servants de canon) | Roscheiderhof, joute Eggenburg, Canon firing | clips NT14 cuits, comparés par `-- measure` | lancé |
| AS8b | chevaux (pas, trot, galop, virage) | Muybridge (DP), Horses running in a pasture (CC BY) | angles suivis → `battle_skinned_gaits.py`, recuit | fait sur `feat/as8b` : trot et galop mesurés (trajectoires de sabots), virages héritent du trot ; pas non refait ; détail `docs/wip/as8b.md` |
| AS8c | bêtes et charrettes de campagne | montbéliardes (CC0), Ploughing, wagons (CC0), moutons, Rama | cadences/amplitudes/phases → `animal_motion.json`, charrettes | lancé |
