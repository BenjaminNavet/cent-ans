# CC C2b — relief des massifs (retour joueur 14/10)

Branche `cc/c2b` (worktree `game_project-cc2b`), issue de `cc-carte-composee`. Module : `compose/relief.py`, réglages `data/map/compose/relief.json`.

## Fait (code)
- Cols = encoches elliptiques à la crête (`carve_passes`, `passes.depth_frac` 0,45 : fond ≈ 55 % de la hauteur locale, largeur ≈ largeur de la chaîne, approche en pente douce). Plus de couloir ; franchissement garanti par `crossings.json` (navgrid).
- Vallées : 4 auges en U d'auteur à fond montant (`troughs` + `trough_walls` : Rhône valaisan, Inn, Adige/Brenner, Isère), jamais plus de 55 % de la hauteur locale creusée ; les fleuves ne creusent plus les massifs (`rivers.climb_limit_m` : tracé coupé à l'amont dès que le terrain dépasse la base de 350 m).
- Masses : largeur variable le long de la crête (`width_variation`), chaînes plus larges (`halfwidth_scale` 0,62), sommets tassés au-dessus de 2 600 m (`compress_above_m`), collines arrondies (`hills.min_halfwidth_px` 16, `halfwidth_scale` 1,5).
- Massif central : plateau d'auteur 880 m (`plateaus`), chaîne ridges.json passée en `hills` (1 100 m).
- Neige : `colormap_style.yaml` 3 050-3 400 m, roche 1 700-2 700 m.

## Reste
Reconstruire (compose, compose-relief, compose-layers), tests, planches `~/dev/cent-ans-raw/compose/c2b/`.
