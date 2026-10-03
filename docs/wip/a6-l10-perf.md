# A6-L10 : appels de dessin au zoom max, profil ultra, fuites

Etat : fait (branche du worktree, voir `git log`). Sonde : `game/tests/a6_drawcalls_probe.gd`
(fenêtrée, 1920x1080 ; `--keep-ui`, `--force` pour descendre sous le plancher de caméra, `--at=x,z`,
`--quality=`, ablation par couche avec part des ombres).

## Mesures (qualité haute, Paris, 1080p)

| distance | appels avant | appels après | primitives avant | primitives après |
|---|---|---|---|---|
| 300 | 1175 | 1175 | 2,88 M | 2,39 M |
| 150 | 1106 | 1106 | 3,17 M | 2,69 M |
| 60 | 882 | 874 | 4,18 M | 3,26 M |
| au plus près (20) | 686 | 678 | 5,33 M | 4,41 M |

Les 6 821 appels de l'audit ne se reproduisent pas : 500 à 1 400 appels dans toutes les vues essayées
(Paris, Orléans, forêt d'Orléans, Londres ; zoom jusqu'à d = 1 ; avec ou sans interface ; haute et ultra).
La cible < 2 500 est tenue sans changement. Le coût réel est en primitives (5 M au plus près).

Ablation au plus près de Paris (appels / primitives) : soleil (passes d'ombre) 350 / 2,9 M ; relief
42 / 1,8 M (dont 0,9 M d'ombres) ; villes 438 / 0,9 M (maquettes 326 / 0,54 M) ; vie (moulins, fumées)
62 / 0,67 M ; rivières 62 / 0,8 M (rivières mineures : 1 appel, 420 k) ; côte 1 / 0,6 M ; armées 14 / 0,25 M.
La végétation est déjà en MultiMesh (92 k arbres en 15 appels).

## Corrections
- Côte : plus d'ombres portées (483 k primitives de passes d'ombre pour un liseré translucide).
- Ailes de moulin : plus d'ombres portées (439 k primitives en 8 passes d'ombre ; MultiMesh de toute la
  carte). Les corps de moulin gardent leur ombre.
- Ultra (`render_quality.gd`) : relief 6 px par sommet (4), ombres arbres/moulins 200 (400), MetalFX
  spatial 0,85 au lieu du natif (-28 % de pixels). Primitives ultra au plus près : 8,4 M -> 6,3 M ;
  zoom 300 : 4,0 M -> 3,5 M.
- Fuite à la sortie (« 60 resources still in use ») : le menu des filtres de `MapModeController` n'entrait
  dans l'arbre qu'à sa première ouverture ; jamais ouvert, il restait orphelin et retenait thème,
  icônes et polices. `_exit_tree` le libère : 253 -> 31 instances d'ObjectDB perdues, plus aucune
  ressource « still in use ».

## Points ouverts
- Instances de MultiMesh de toute la carte (moulins, cheminées, rivières mineures, côte) : leurs
  sommets sont traités même hors champ ; les découper par tuile gagnerait encore ≈ 1 M de primitives.
- Ombres des maquettes (149 appels, 367 k) et du relief (0,9 M) : réglables par `shadow_range`
  (`data/art/town_maquettes.json`) et `relief_shadow_cascades`.
- Coût GPU réel d'ultra non mesuré (machine chargée) ; 31 instances encore perdues à la sortie.
