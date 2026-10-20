# TS-T10 — plaques de neige d'hiver

Branche `ts/t10`. Fichiers : `terrain.wesl` (neige), `season.rs`, `data/ui/campaign_seasons.json` (+ schéma).
Avant : `plain_share` 0,55, pas de variation régionale, octave grossière jamais estompée.
État : plain_share 0,93, `snow.region_variation` (snow_plain.w), octave grossière estompée avec la distance. Réglage à l'œil en cours.

## Final
Réglages : plain_share 0,55 -> 0,93 ; region_variation (nouveau) 0,18 ; forest_share 0,7 -> 0,92 ; mud_fringe 0,18 -> 0,10 ;
bord de neige smoothstep(0.5,0.6) -> (0.42,0.68) (constante shader) ; octave grossière estompée avec l'empreinte (1,5..5).
Vérifs : fmt, clippy, tests 1335, smoke 10/10. Capture : /Users/jean_hubert/dev/gp-ts-t10-shots/b_s.jpg.
Reste : taches brunâtres moyennes encore visibles en plaine à gauche (frange de forêts) ; bord doux/trou en constantes shader.
