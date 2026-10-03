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

(Section rédigée par le lot RJ-d.)
