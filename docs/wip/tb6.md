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
- [x] 5. Retouches : automne, massifs forestiers, brume en nappes larges (voir plus bas).
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

## Retouches après capture de contrôle (02/10)
Mesures par `tb6_shot.gd` (rendu forcé avant chaque lecture d'image : une fenêtre occultée ne
fige plus la mesure), Paris, `--map-weather=clear` sauf pour la brume.
- Automne orange vif : soleil d'automne moins chaud (`sun_color` 1,0 / 0,89 / 0,74, énergie 1,3,
  diffusion 0,34) et étalonnage de carte d'automne qui retient le rouge (`gain` 0,95 / 1,03 / 1,0,
  saturation 0,7), dans `data/fx/atmosphere.json`. À 400 : automne teinte 28° → 32°, saturation
  0,572 → 0,526 (été : 44°, 0,549) ; à 90 : 27° → 31°, 0,514 → 0,481 (été : 38°, 0,508).
- Massifs forestiers : la source est la carte de couleur (blocs sous 40 % de la médiane à 400 :
  8,0 % avec, 2,1 % sans). Surcouche dans `sg_apply` (`satellite_ground.gdshaderinc`, uniformes
  `sg_dark_*`, bloc `forest_masses`, posé par `MapReadability.apply_forest_masses`). Blocs sous
  40 % de la médiane, été : 1100 : 2,9 % → 0,3 % ; 400 : 8,0 % → 0,6 % ; 250 : 0,3 % après.
  Hiver : 1,6 % → 0,3 % et 1,9 % → 0,2 %. 5e centile rapporté à la médiane (été) : 0,45 → 0,64
  (1100), 0,34 → 0,56 (400).
- Brume en veines : la nappe suit désormais le relief lissé (niveau 4 contre niveau 7 de la carte
  des hauteurs), plus les plaines basses, hors crêtes, en bancs qui dérivent ; opacité max 0,32.
  `--season=autumn --map-weather=fog --ab` : largeur des nappes pondérée par la surface 182 px
  carte à 1100, 93 à 400, 62 à 90 (plus large : 712 / 258 / 157) ; sol éclairci de plus de 10 % :
  35 % / 17 % / 17 % ; rapport de luminance au 99e centile 1,52 / 1,34 / 1,16 ; province
  sélectionnée : 1,000 à 1,004.

## Prochaine étape
Capture de contrôle par la session principale (les agents ne lisent pas d'image) :
`godot --path game --resolution 1600x900 --script res://tests/tb6_shot.gd -- --out=<dossier>
--season=summer --distances=1100,400,90` (Paris `2213.2,3203.9`, météo du cœur) ; brume :
`--season=autumn --map-weather=fog --prefix=brume-` puis avec `--select` ; hiver :
`--season=winter`. Pour une vallée plus marquée que la Seine, passer `--at=<x>,<z>` (px carte).
Réglage à l'œil : `morning_mist` (`opacity` 0,32, `plain_share`, `bank_scale`), `forest_masses.floor`
(0,12 ; 0,10 plus sombre, 0,16 plus clair), couleurs et énergie du soleil.

## Points ouverts
- Banc à refaire sur une machine calme (`tb6_shot.gd --bench --disable-vsync --map-weather=fog`) :
  le seuil de 1 ms du SSIL est sous le bruit de mesure d'aujourd'hui.
- Les mesures `--ab` d'avant la retouche (sans rendu forcé) portaient un bruit de ± 10 % et
  parfois des images figées ; celles des retouches sont fiables à ± 1 %.
- La teinte de sol procédurale des forêts (`tint_forest`, assombrissement vers
  `terrain.gdshader:633`) n'est pas touchée : de près (rig < 250), les blocs très sombres sont
  déjà sous 0,3 %. À revoir avec HC quand les arbres grossis seront posés.
- `campaign_map.gd` touché d'une ligne (`apply_forest_masses`) : conflit possible à la fusion.
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
