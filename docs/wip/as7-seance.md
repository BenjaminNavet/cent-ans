# AS7 — séance de jugement des animations en jeu (15 min)

Le joueur regarde, juge ; rien à coder pendant la séance. Notes en face de chaque case, puis
reporter les « à reprendre » dans `docs/wip/as8.md`. Options vérifiées par grep dans `game/` le 09/10.

## Lancements (deux seulement)
- **B** (bataille seule, France–Angleterre, IA des deux camps) :
  `godot --path game res://scenes/battle/battle.tscn -- --autoplay --units=8`
  (`--closeup`, `--weather=clear` inutiles ici ; molette = zoom, rester à 15-40 m de la mêlée / de la colonne).
  Vitesse : touches − / + (ralenti ×0,5 possible) ; pause pour regarder une pose.
- **C** (campagne) : `tools/launch.sh -- --autostart=fac_france`, puis zoom maximal sur une armée en marche
  et sur la campagne (champs, routes).
- Un agent qui veut lancer l'un des deux avec fenêtre passe par `tools/godot_bg.sh --path game … -- <options>`.

## Check-list (du plus important au moins important)
| # | À regarder | Comment | Attendu | Verdict |
|---|---|---|---|---|
| 1 | Marche de l'infanterie à l'épée | B, premières secondes, lignes qui avancent, caméra basse ¾ | pas calés sur le sol (pas de patinage), cadence naturelle, foule désynchronisée | OK / à reprendre / note : |
| 2 | Course et charge à pied | B, quand les lignes se rejoignent (charge = `run` ×1,05, pas de clip dédié) | allure rapide crédible, pieds posés ; noter si « une charge » ne se distingue pas d'une course | OK / à reprendre / note : |
| 3 | Mêlée (estoc, parade, garde, coup) | B, au contact, zoom 15 m | gestes variés, lame d'estoc pas trop haute (connu 20-40°), pas de saut entre clips | OK / à reprendre / note : |
| 4 | Pas du cheval (`c_walk`) | B, cavalerie qui avance au pas (début de bataille) | connu : 49 % de patinage, sabots qui glissent ; juger si gênant à distance de jeu | OK / à reprendre / note : |
| 5 | Trot Muybridge (`c_trot`) | B, cavalerie en approche moyenne vitesse | 4 temps lisibles, sabots posés (patinage 5 %), cadence 3,4 | OK / à reprendre / note : |
| 6 | Galop Muybridge (`c_gallop`) | B, cavalerie lancée / charge | suspension visible, foulée ni trop courte ni trop vive (cheval du jeu plus petit : point ouvert AS8b) | OK / à reprendre / note : |
| 7 | Virages de cavalerie (pas, trot) | B, escadron qui change de direction | inclinaison du cheval, pas de glissade latérale ; les pivots d'infanterie glissent (connu) | OK / à reprendre / note : |
| 8 | Archers `bow_walk` | B, archers qui avancent (branché dans `STYLES`, repli `walk`) | marche arc en main distincte de la marche épée ; si identique : clip absent en jeu | OK / à reprendre / note : |
| 9 | Arbalétriers `xbow_walk` | idem | arbalète portée, marche distincte | OK / à reprendre / note : |
| 10 | Tir arc et arbalète | B, pause à la salve, zoom 15 m | encocher/bander/lâcher lisibles, flèche part de l'arc | OK / à reprendre / note : |
| 11 | Piquiers (`pike_walk`, `pike_level_walk`, `brace`) | B, avance puis abaissement face à la cavalerie | pique horizontale à l'abaissement, genoux pas rentrés (connu KayKit) | OK / à reprendre / note : |
| 12 | Déroute et morts | B, fin de bataille | fuite, chutes variées, renversés qui glissent un peu (connu) | OK / à reprendre / note : |
| 13 | Armées en marche sur la carte | C, zoom max sur une armée qui se déplace | pas calé sur la vitesse (AS2), arrêt = idle | OK / à reprendre / note : |
| 14 | Bovins et chevaux de camp (AS8c) | C, zoom max près d'une ferme / d'un camp | foulée des bovins plus vive (1,4 → 0,8-0,86 m), tête qui broute, queue ; pas de glissade | OK / à reprendre / note : |
| 15 | Charrettes et chariots (AS8c) | C, zoom max sur route, charrette en mouvement | cahot en 3 raies, roulis léger, roues cohérentes avec l'avance | OK / à reprendre / note : |
| 16 | Moutons | C, zoom max sur pâture | broutage seulement (foulée non mesurée, point ouvert) | OK / à reprendre / note : |
| 17 | Flammes et bannières de carte (AS5) | C, zoom moyen sur une ville / maquette | scintillement du feu, onde de vent des bannières | OK / à reprendre / note : |
| 18 | Trébuchet et servants (AS8d) | B avec `--siege` : `godot --path game res://scenes/battle/battle.tscn -- --siege --autoplay` | bras du trébuchet suivant la courbe mesurée, servants qui s'activent | OK / à reprendre / note : |

## A/B possibles (options qui existent encore)
- `-- --keyframed-melee` : mêlée keyframée au lieu des gestes vidéo NT14 (poste 3).
- `-- --fa-anim` : active tous les clips FA3 retargetés (défaut : 3 seulement) (postes 1-3).
- `-- --mocap-trial` : essai CMU (NT12), à ne regarder que si le joueur est curieux.
- `-- --coarse-figures` / `--legacy-figures` : kit grossier / figurines rigides, pour comparer.
- Campagne : `-- --legacy-army-markers` (anciens marqueurs d'armée), `-- --life-off=<liste>` / `--folk-off=<liste>` pour couper des couches de vie.
- Chevaux : l'A/B AS3 contre AS8b est `AS8B_LEGACY=1` côté Blender (recuit), pas en jeu.

## Options citées dans les notes qui n'existent plus
- `--no-as5` (A/B AS5, `docs/wip/as5.md`) : absent de `game/`.
- `--no-fa-anim` : absent (seul `--fa-anim` existe).
- Aucune option d'A/B en jeu pour AS8c ni AS8d (bêtes, charrettes, trébuchet).
- `--free-move` : absent.

## Écart avec l'audit
`14-animation.md` et `13-rigger.md` disent `bow_walk`/`xbow_walk` non branchés : c'est faux dans le code
(`battle_skinned.gd` `STYLES`, repli `walk`, clips présents dans `battle_fine/manifest.json`). Ce qui reste
à établir en séance : si le clip est bien lu (postes 8-9).

## Après la séance
Reporter dans `docs/wip/as8.md` : verdicts, puis ouvrir au besoin : pas du cheval (poste 4), charge à pied
(poste 2), tournage AS6 (`15-mocap.md`, prises 1-5).
