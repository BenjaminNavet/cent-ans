# ADR 0175 — Lisibilité du territoire : possession, occupation, position

État : accepté (lots RJ-c et RJ-d, chantier `docs/wip/rj-retours-joueur.md`).

## Contexte

Retours joueur du 03/10 : en campagne, on ne sait pas ce qui est à nous et ce qui est aux autres ; la prise d'une ville et la prise d'une région se confondent (« comment je prends possession d'une région ? »). Décision du joueur : la règle de conquête reste (la cité d'une province en donne le contrôle ; la possession ne change que par traité, par cession ; à la paix, les places occupées et non cédées sont rendues), mais elle doit être expliquée ; la carte gagne un remplissage par position diplomatique (vue 3D) et des bannières sur les villes.

## Explication (RJ-c)

Décision :
- Le cœur classe une place ou une province pour un observateur (`sim_campaign::possession`) : `own`, `own_occupied`, `occupied_by_viewer`, `foreign`, `foreign_occupied`, avec le nombre de places tenues sur le total et le détenteur de la province entière (bonus de province complète). Le pont l'expose (`CampaignSim.province_possession(id, viewer)`, `settlement_possession(id, viewer)`, `viewer` vide = le joueur). L'interface ne recalcule rien : elle met en mots (`PossessionText`).
- Statut en une ligne, coloré selon la position (ADR 0155 ; encre foncée lisible sur parchemin) : « À vous », « À vous — occupée par X », « Occupée par vous (propriétaire de droit : Y) — rendue à la paix si non cédée », « À X », « À X — occupée par Z ». Il remplace la ligne « Propriétaire » des panneaux de province et de colonie (clé « Statut »), suit le nom de la province survolée sur la carte (filets latéraux à la couleur de position) et porte une infobulle IB (`province_possession` de `tooltips.json`).
- Panneau de province : « Places tenues : n sur N » (bonus de province complète acquis, ou places manquantes) ; l'onglet Colonies commence par la phrase d'aide « La cité de <nom> donne le contrôle de la province ; la possession s'obtient par traité (cession) » et chaque place occupée porte « occupée par … » ; la cité est signalée « (cité : donne la province) ».
- Panneau de colonie : aide selon que la place est la cité (qui la tient contrôle la province) ou une autre place (compte pour le bonus de province complète seulement), plus la restitution à la paix si elle est occupée.
- Fenêtre de capture : titre « X est prise » (et non plus « est à vous ») ; le texte du cœur ajoute une ligne (`capture::occupation_reminder`) : « Occuper n'est pas posséder… », précédée de « C'est la cité de … : vous en prenez le contrôle » pour une cité, ou remplacée par « vous la recouvrez » pour une place qui nous appartient de droit.
- Codex, onglet Mécaniques : fiche « Conquête et possession » (cité et autres places, occupation et possession, cession par traité, restitution à la paix, bonus de province complète).

Conséquences : aucun changement de règle ; les textes de statut vivent dans `game/scripts/ui/possession_text.gd`, la classification dans le cœur.

## Bannières (RJ-c)

Décision :
- L'écu posé au-dessus du nom (`settlement_icon.gdshader`, un seul `MultiMesh` pour tous les lieux, atlas `HeraldryAtlas` des armoiries existantes) est désormais celui du **propriétaire de droit**, et non plus du détenteur. Une place occupée élargit son quad et ajoute, en bas à droite, à demi posé sur le premier, le petit écu de l'**occupant**. Propriétaire sans armoiries : l'écu de l'occupant seul.
- Chaque écu porte un liseré à la couleur de position du joueur (soi or, ami vert, ennemi rouge : couleurs de `stance_cues.json`) ; aucun pour une faction neutre, qui garde ses seules armoiries. Le liseré est une dilatation de l'alpha de l'écu dans le shader (8 prélèvements, niveau de mipmap calculé hors des branches).
- Encodage par instance sans nouveau canal : COLOR.r = case du propriétaire, COLOR.g = (case de l'occupant + 1) × 16 + position du propriétaire × 4 + position de l'occupant. Les autres canaux (dé-encombrement DA7d, surbrillance SA) sont inchangés ; le rectangle de dé-encombrement s'élargit avec la bannière.
- Masquage selon le zoom inchangé (rang et distance, `SettlementMarkers`), sauf `shield.max_distance_by_rank` du rang 2 (cités et places majeures) porté de 330 à 700 : l'écu suit désormais le nom des cités en vue moyenne.
- Parchemin : chaque cité porte un fanion à la couleur héraldique du propriétaire, liseré de position ; place occupée : fanion de l'occupant sous le premier, sur la même hampe.
- Mise à jour à chaque `refresh_all` (capture, traité, déclaration de guerre) : seules les instances dont la bannière change sont réécrites ; l'atlas n'est recomposé que si une faction nouvelle apparaît. Paramètres : `data/map/settlement_markers.json` (`banner`), schéma `settlement_markers.schema.json`.

Conséquences : aucune ressource nouvelle ni génération payante ; coût d'un rafraîchissement complet mesuré par `game/tests/rj_possession_test.gd` (≈ 2 100 lieux) ; coût GPU négligeable (quads de quelques dizaines de pixels).

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
