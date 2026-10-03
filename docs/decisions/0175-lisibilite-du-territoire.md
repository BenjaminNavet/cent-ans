# ADR 0175 — Lisibilité du territoire

Date : 2026-10-03. Statut : acceptée. Chantier RJ (`docs/wip/rj-retours-joueur.md`).

Retour joueur du 03/10 : en campagne, on voit mal comment on progresse ; son territoire est
difficile à délimiter ; possession et occupation se confondent. Décision du joueur : la règle de
conquête est gardée mais expliquée (lot RJ-c), et la lecture passe par un remplissage par
position diplomatique en vue 3D (lot RJ-d) et des bannières sur les villes (lot RJ-c).

## Remplissage par position (RJ-d)

Révise le point 4 de l'ADR 0124 (« couleur politique neutralisée en vue normale ») : la vue 3D
retrouve une lecture politique, mais par **position envers le joueur**, non par couleur
héraldique. La couche « politique » héraldique du terrain (`political_amount`,
`faction_alpha_near`) reste telle quelle.

### Contexte
En vue 3D, la teinte de faction est à 0,04 (ADR 0124, HB) : seules les frontières disent à qui
est quoi. Les couleurs héraldiques de 177 factions ne se distinguent pas d'un coup d'œil, et le
rouge reste réservé aux ennemis (ADR 0155).

### Décision
- **Lavis translucide de chaque province** selon la position de son **contrôleur** envers le
  joueur : or = nous, vert = alliés et vassaux, rouge = ennemis en guerre, gris très léger = les
  autres. Catégories et couleurs reprises de `StanceCues` / `stance_cues.json` (pas de seconde
  classification, couleurs identiques aux frontières).
- **Contrôleur plutôt que propriétaire de droit.** Le joueur veut voir sa progression : une cité
  prise fait passer sa province à l'or dès la prise, alors que la possession ne change qu'au
  traité (règle de conquête gardée). La possession de droit reste lisible sans rien ajouter :
  trait de frontière à la couleur du propriétaire et hachures de l'occupant le long du bord
  (FR1, ADR 0074), toutes deux peintes **après** le lavis, donc jamais masquées. Province
  ennemie prise : lavis or, bord rouge hachuré d'or ; province à nous occupée : lavis rouge, bord
  or hachuré de rouge. Les bannières de RJ-c redisent possesseur et occupant sur la ville.
- **Teinte à luminance conservée** (comme la teinte de faction du terrain) : le relief, les
  forêts et les champs gardent leur valeur, seule la couleur glisse. Poids (`alpha`) : nous
  0,45, ennemi 0,42, ami 0,35, autres 0,10, part de chromaticité 0,8. Réglés sur capture : les
  valeurs de départ (0,15-0,25) ne déplaçaient le rendu que de 3/255 en moyenne, illisible sur
  un sol vert ; la teinte ne change que la chromaticité, d'où des poids plus hauts qu'un
  mélange alpha ordinaire.
- **Atténuation de près** par la distance caméra (fondu entre 60 et 260, × 0,35 de près) pour
  laisser villes et champs à leur rendu. Distance caméra plutôt qu'empreinte pixel : indépendant
  de la résolution (ADR 0123).
- **Parchemin** : même lavis, opacité × 0,55 (`parchment_scale`) par-dessus le lavis héraldique,
  pour que les deux vues disent la même chose sans salir l'aquarelle.
- **Mode politique seulement** (`modes`) : Diplomatie, Religion, etc. peignent déjà leurs
  propres couleurs.
- **Option du joueur** `map/stance_fill` (Réglages › Carte, « Lavis des positions
  diplomatiques »), activée par défaut ; `--no-stance-fill` en ligne de commande pour l'A/B.

### Mise en œuvre
- `game/scripts/map/stance_fill.gd` (`StanceFill`) : texture 1 × N (index raster de province)
  refaite seulement si contrôleurs ou positions changent, à chaque `refresh_all` (instantané
  `ProvinceSnapshot` partagé) ; aucune reconstruction du terrain.
- `game/shaders/stance_fill.gdshaderinc`, crochet `sf_fill` d'une ligne juste avant
  `fr1_borders` dans `terrain.gdshader` et `terrain_parchment.gdshader` ; `sf_alpha` = 0 coupe
  tout au premier test uniforme.
- Réglages : `data/map/stance_fill.json`, schéma `data/schemas/stance_fill.schema.json`.
- Tests : `game/tests/rj_stance_fill_test.gd`, `tools/tests/test_stance_fill_schema.py` ;
  captures `game/tests/rj_fill_shot.gd`.

### Conséquences
- Coût par fragment : une lecture de texture 1D et quelques opérations, sur un shader de terrain
  qui en fait des dizaines ; rien côté CPU hors `refresh_all`.
- Les neutres en paix restent sans couleur propre : leur identité passe par la frontière
  héraldique assourdie (ADR 0155) et le parchemin.
- Le lavis dit la situation militaire (contrôle) ; la situation de droit se lit au bord des
  provinces et sur les bannières.
