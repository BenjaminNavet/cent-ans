# TB6 — lumière et atmosphère (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB6 ». Branche `feat/tb6`, worktree
`/Users/jean_hubert/dev/gp-tb6`. Rendu Godot seulement (`core/` intact). ADR 0156.

## État
- [x] 1. Taches sombres : cause = masque météo du sol (`weather_ground` de
      `campaign_weather.gdshaderinc`) : sol mouillé −28 % × ombre des nuées −35 % à bord net, par
      paliers d'un tiers au bord des provinces, sous un plan de nuées presque invisible à 400. Le
      cœur donne pluie ou orage sur 145 provinces sur 443 au tour 1 (Paris : brume) : le temps
      n'était pas clair, il le paraissait. Désormais −4 % / −6 % (large, fondue) / −5 % (ombres de
      nuages du terrain), au plus −14,3 % ; rien par temps clair. Bloc `clouds` de
      `data/ui/campaign_map.json` (`wet_dim`, `mask_shadow`, `mask_shadow_softness`,
      `mask_shadow_scale`, `weather_shadow`).
- [x] 2. Lumière dorée : `data/fx/atmosphere.json`, bloc `campaign` (soleil 20° / 21° / 18° / 18°,
      plus chaud et plus fort sauf l'hiver, diffusion vers le soleil relevée).
- [x] 3. SSIL gardé (déjà actif en Haute et Ultra), SDFGI écarté : ADR 0156.
- [x] 4. Brume du matin : nappe de vallée dans le shader du sol (`weather_valley_*`, bloc
      `morning_mist`), jamais sur la province sélectionnée.
- [x] `game/tests/tb6_light_test.gd`, `game/tests/tb6_shot.gd`.

## Chiffres (02/10, machine chargée, charge 30 à 80)
- Écart avec / sans masque météo du sol, Paris, été, météo du cœur (`tb6_shot.gd --ab`, part des
  blocs de sol assombris de plus de 15 %, 1er centile du rapport de luminance) : à 400, 29,3 % et
  0,61 avant, 0,1 % et 0,93 après ; à 1100, 7,8 % et 0,75 avant, 0,4 % et 0,93 après (bruit de
  fond de la mesure : 0,91 à 0,94).
- `ss_shot.gd --stats --season=summer` (moyenne RVB / écart-type, tiers bas) : avant 1100 :
  88 79 49 / 52 38 29, 400 : 79 71 40 / 46 33 21 ; après le point 1 : 92 82 52 / 52 38 30 et
  93 84 45 / 51 36 22 ; après tout le lot : 90 78 47 / 53 38 28 et 91 80 43 / 50 35 21. L'écart-type
  absolu ne baisse pas (il vient du contenu : forêts, ville, parcelles) ; rapporté à la moyenne il
  passe de 0,59 à 0,57 (1100) et de 0,58 à 0,55 (400). Avec `--map-weather=clear` : 85 74 41 /
  55 39 27 et 90 78 37 / 54 38 22.
- Hiver après le lot (inchangé, bleuté) : 1100 : 99 102 107, 400 : 75 75 77, 90 : 78 79 82.
- Brume (`--map-weather=fog --season=autumn --ab`, calque `valley_mist`) : blocs éclaircis de plus
  de 10 % : 27 % à 1100, 10 à 19 % à 400, 12 % à 90 ; province sélectionnée (`--select`) : rapport
  moyen 1,001 à 1,008.
- Banc : voir l'ADR 0156. `ss_shot.gd --bench` (1100 / 400 / 90, ms par image) : avant 16,68 /
  6,90 / 7,87 puis 14,06 / 18,41 / 19,67 (charge 28) ; après 23,92 / 30,45 / 32,15 puis 24,47 /
  27,84 / 30,33 (charge 37 à 81). Non comparables : la charge a doublé entre les deux séries.
  L'écart mesuré dans le même processus est dans le bruit pour la brume.

## Prochaine étape
Capture de contrôle par la session principale (les agents ne lisent pas d'image) :
`godot --path game --resolution 1600x900 --script res://tests/tb6_shot.gd -- --out=<dossier>
--season=summer --distances=1100,400,90` (Paris `2213.2,3203.9`, météo du cœur) ; brume :
`--season=autumn --map-weather=fog --prefix=brume-` puis avec `--select` ; hiver :
`--season=winter`. Vallées marquées : Seine en aval de Paris, Meuse `--at=2560,3020` (à vérifier).
Réglage à l'œil : `morning_mist.opacity` (0,55), `valley_depth_m` ([20, 90]), couleurs et énergie
du soleil.

## Points ouverts
- Banc à refaire sur une machine calme (`tb6_shot.gd --bench --disable-vsync --map-weather=fog`) :
  le seuil de 1 ms du SSIL est sous le bruit de mesure d'aujourd'hui.
- Fenêtre occultée par une autre : l'image ne se redessine plus, `--ab` rend 1,00 partout.
  Relancer avec la fenêtre visible.
- Brouillard de guerre (TB2) : provinces hors de vue jusqu'à −35 % à bord net ; autre source
  possible de « taches sombres » sur la capture, hors de ce lot.
- La nappe de brume est peinte sur le sol : arbres et villes n'en sont pas voilés. Avec la carte
  des hauteurs en repli LA8 (sans niveaux grossiers), pas de nappe.
- Les paliers d'un tiers du masque météo au bord des provinces (`wx_sample`) restent ; ils ne
  pèsent plus que 1 à 2 % de luminance.
- `tb2_declutter_test` : un échec isolé sur trois lancements (« set_limoges cut by the screen
  edge », étiquettes, machine chargée) ; sans rapport avec TB6.
- `terrain.gdshader` touché en deux endroits (appel de `weather_ground`, normale de fin) : conflit
  possible avec TB4 / TB5 à la fusion.
