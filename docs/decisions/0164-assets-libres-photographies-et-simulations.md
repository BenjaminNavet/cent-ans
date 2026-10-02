# ADR 0164 — Assets libres : matière photographiée ou simulée, recodée au format du jeu (FA)

Date : 2026-10-02. Statut : accepté. Chantier FA (`docs/wip/fa.md`), 0 $.

## Contexte
Le joueur demande de chercher des assets gratuits et de les utiliser s'ils améliorent le rendu,
puis d'étendre la recherche aux animations, à la campagne et à l'interface. Le dépôt est public :
tout fichier versionné est redistribué. Plusieurs éléments visibles restaient dessinés par code
(feuilles des arbres de bataille, touffes d'herbe, flammes et fumée, ornements de la vue
parchemin, croix et boutons plats de l'interface) ou animés à la main.

## Décision
1. **Licences admises** : CC0 et domaine public ; CC BY accepté avec crédit. Jamais NC, ND, SA ni
   « pas de redistribution ». La licence est vérifiée à la source (page de licence, API du musée,
   `extmetadata` de Wikimedia Commons, fichier de licence du pack) et citée dans `CREDITS.md` et
   dans le `SOURCE.md` du dossier.
2. **Les brutes restent hors dépôt** (`~/dev/cent-ans-raw/fa/`). Le dépôt ne reçoit que des dérivés
   découpés et recodés, produits par un script versionné qui télécharge la source au besoin et lit
   un catalogue de données (`data/art/`, `data/fx/`, `data/map/`, `data/ui/`) validé par un
   schéma : aucune coordonnée ni identifiant de source dans le code.
3. **Recodage au format existant plutôt que nouveau rendu** : les nouvelles textures gardent la
   convention des shaders en place (rameau alpha par essence, touffe en luminance et alpha,
   planches 8×8 chaleur / densité), si bien que le gain vient de la matière et non d'un nouveau
   chemin de rendu.
4. **Un lot n'entre que s'il bat l'existant sur capture avant/après**, à plusieurs distances. Ce
   qui est moins bon est écarté même s'il est « vrai » : navire de l'Atlas catalan (illisible à
   l'échelle de la carte), lettrine et rinceau réels (flous à 50 px), dix clips d'animation sur
   treize (équivalents ou moins bons).
5. **Chaque lot garde un drapeau d'A/B** après `--` : `--no-fa` (feuillages, interface),
   `--no-fa-grass`, `--no-fa-parchment`, `--no-fa-anim` / `--fa-anim`.
6. **Compression** : textures de végétation en compression haute qualité (BC7 / ASTC) avec
   mipmaps ; le S3TC simple donne des reflets vert fluo sur les feuillages sombres.

## Retenu
| Lot | Source | Usage |
|---|---|---|
| FA1 | ambientCG LeafSet (CC0) | rameaux des arbres de bataille, un par essence |
| FA2 | Unity Labs, flipbooks VFX (CC0) | flammes (Flame03) et fumée (WispySmoke03) |
| FA3 | Mesh2Motion, KayKit (CC0) | parade et deux morts par défaut ; dix autres clips derrière `--fa-anim` |
| FA5 | Met Open Access, ambientCG (CC0) | sceaux de cire, boutons cuir et laiton de l'accueil |
| FA6 | Atlas catalan de 1375 (domaine public) | rose des vents et sirènes de la vue parchemin |
| FA7 | ambientCG Foliage (CC0) | herbe des batailles, atlas de douze touffes |

## Conséquences
- Poids ajouté au dépôt : quelques mégaoctets de textures dérivées ; aucune dépense.
- Toute nouvelle source suit le même chemin : catalogue, script, `SOURCE.md`, crédit, capture
  avant/après, drapeau d'A/B.
- Lacunes connues, sans source libre téléchargeable sans compte : cogue médiévale, feuillus
  européens en 3D, vrai parchemin tuilable, nuages tuilables. Les rochers photogrammétrés de
  Poly Haven sont proposés aux sessions qui tiennent la carte 3D.
- La carte de campagne 3D n'est pas touchée par FA (sessions TB, GC, HC).
