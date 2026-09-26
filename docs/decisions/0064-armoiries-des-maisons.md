# 0064 — Armoiries des maisons et héraldique des figurines (DA1)

Date : 2026-09-26. Statut : accepté. Lot DA1 (bible DA, § 1-2, 6, 7 ; écart n° 2 du § 10).

## Contexte

L'héraldique fait le lien entre les deux registres du jeu, l'enluminure en 2D et le semi-réalisme
en 3D. Jusqu'ici les armoiries n'existaient que par faction (29 écus). Les 51 maisons des
personnages n'avaient pas d'armes, et les figurines skinnées (V2) ne portaient les armes de leur
faction que sur l'écu, le pavois et le caparaçon. Le surcot restait d'une livrée unie.

## Décision

1. **Données** : `data/heraldry/houses.json` (schéma `heraldry_houses.schema.json`) contient une
   entrée par valeur du champ `house` des personnages :
   - le blasonnement français, avec source et degré de certitude (`attesté`, `incertain`,
     `substitution`) ;
   - `arms_of` pour les 13 maisons dont les armes sont celles d'une faction (dessin identique) ;
   - `vassal_of`, les armées où servent ses bannerets ;
   - `badges`, les croix de livrée du commun (croix blanche de France, croix de saint Georges
     sur pièce blanche, etc.).
2. **Dessin** : `heraldry.py` garde sa grammaire d'origine pour les factions. Leurs écus restent
   identiques à l'octet, ce que vérifient les tests. Une grammaire v2, réservée aux maisons,
   ajoute :
   - toutes les teintes lues dans le blason ;
   - les champs divisés (bandé, burelé, échiqueté, fuselé, coupé, écartelé en sautoir, semés) ;
   - l'écartelé récursif ;
   - pièces et meubles dans l'ordre du texte (nombre = mot précédent, teinte = première teinte
     qui suit, `chargé de`, `accompagné de`, `en fasce`) ;
   - les fourrures (hermine, vair) et les motifs (componé, échiqueté).

   Sortie : `game/assets/heraldry/houses/<id>.png` (128², mêmes finitions que les écus de faction).
   Commande : `cent-ans assets heraldry`.
3. **Interface** : `HouseArms` lit les données. `PortraitLoader.house_heraldry_texture(house,
   faction)` se replie sur les armes de la faction. Les armes de la maison remplacent celles de
   la faction :
   - fiche de personnage (écu du portrait, info-bulle du blason) ;
   - cour et arbre familial (quand il n'y a pas de portrait) ;
   - sceau du général ;
   - chef du HUD de bataille et général du dialogue d'avant-bataille.
4. **Figurines** :
   - `BattleSoldiers` construit une fois par bataille un **atlas `Texture2DArray`**. Il contient
     les armes des deux factions, des maisons des deux généraux et de leurs maisons vassales, soit
     24 couches pour France-Angleterre, 128² avec mipmaps, environ 2 Mo.
   - Chaque matériau de régiment reçoit `arms_layers` : le seigneur plus trois bannerets tirés
     par une graine déterministe, l'id d'unité.
   - Le shader skinné choisit la couche de chaque figurine en hachant son `INSTANCE_ID` : 30 %
     de bannerets chez les nobles (`vassal_share`). Il n'y a ni donnée par instance, ni texture
     par figurine, ni coût CPU par image.
   - Les nobles (`noble` du manifeste) portent ces armes sur l'écu, le caparaçon, et la poitrine
     et le dos du surcot. Cette projection se fait en pose de repos, dans une boîte du buste
     mesurée au chargement (sommets livrée des os Torso/Chest). Le commun garde la livrée unie
     avec la croix de sa faction.
   - Le shader rigide de repli reçoit les armes de la maison du général pour les unités nobles.
   - Les armoiries ne sont plus grisées : teintes saturées, variation de ±6 % (bible § 2, § 3.1).
   - `--no-da1` rétablit le rendu d'avant pour les mesures A/B.

## Conséquences

- Tout nouveau personnage historique dont la maison est nouvelle doit recevoir une entrée dans
  `houses.json`. Un test pytest échoue sinon. Les personnages nés en jeu héritent de la maison
  de leur père, donc de ses armes.
- Les maisons royales portent leurs armes de lignage : Valois à la bordure, Plantagenêt
  écartelé. Les armes du royaume restent celles de la faction, sur les bannières et les pennons.
- Trois maisons ont des armes de substitution (Artevelde : Gand ; Béhuchet : Le Mans ; Le Bel :
  perron de Liège) et une a des armes incertaines (Petrarca). Chacune est signalée dans les
  données et dans l'info-bulle.
- Limites :
  - les cadavres, partagés par camp, prennent les trois premiers bannerets du camp ;
  - les imposteurs lointains cuisent le matériau d'un régiment ;
  - les étendards (EP5) portent encore les armes de la faction.
- **Perf** : le banc de bataille (`--benchmark`) est trop bruité sur la machine partagée pour
  voir 3 %. Sur 10 paires alternées, les écarts vont de 35 à 93 i/s d'une passe à l'autre, et
  les dernières paires, machine calme, donnent 90-93 i/s dans les deux cas. La mesure retenue
  vient de `res://tests/da1_perf.gd` : une foule skinnée en gros plan, en un seul processus, où
  les rendus DA1 et shader de main (`--base-shader=`) alternent toutes les 30 images. Écart
  mesuré : +0,22 %, -0,17 % (29,7 ms par image, `--density=3`) et +0,03 % (plafond à 60 Hz,
  `--density=2`). Coût négligeable, bien sous le seuil de 3 %.

## Révision DA1b (2026-09-26) : meubles dessinés et étendards aux armes du général

Contexte : les lions, léopards, aigle, guivre, dauphin et château étaient des silhouettes
polygonales en blocs, visibles sur une vingtaine d'écus (Angleterre, Empire, Flandre, Bohême,
Écosse, Plantagenêt, Lancastre, Luxembourg, Visconti…). Les étendards EP5 portaient encore les
armes de la faction.

Décisions :

1. **Meubles vectoriels du domaine public** : 12 SVG de Wikimedia Commons (série « Meuble
   héraldique » du projet Blasons, armes Visconti de 1395), licence vérifiée page par page :
   domaine public, CC0 ou CC BY 4.0, aucun CC BY-SA. Ils sont vendus dans
   `data/heraldry/charges/` avec `SOURCE.md` (auteur, lien, modification) et crédités dans
   `CREDITS.md`. Le manifeste `charges.json` (schéma `heraldry_charges.schema.json`) donne à
   chaque couleur du dessin un rôle : corps, ombre, armé (griffes, langue, bec, nageoires),
   couronne, ouvertures, issant, argent, trait.
2. **Recoloration exacte** : `heraldic_charges.py` rend chaque SVG avec `resvg` (roue
   `resvg-py`, déterministe) une fois par rôle, couleurs du rôle en blanc et le reste en noir.
   Le niveau de gris donne la couverture du rôle, anticrénelage compris. Les masques sont
   recadrés et mis en cache, puis peints avec les teintes du blason : `armé/lampassé/becqué/
   peautré de …` (gueules par défaut, azur sur un meuble ou un champ de gueules), `couronné de …`
   (or par défaut), `ouvert et ajouré de …` (sinon le champ transparaît). Un meuble de sable a
   des traits clairs. Le trait est épaissi d'un pixel de rendu pour survivre à la réduction en
   128².
3. **Variantes lues dans le texte** : lion couronné, lion à la queue fourchée (Bohême), queue
   fourchée passée en sautoir (Luxembourg), lion ailé de saint Marc (Venise), léopard lionné
   (Armagnac). Une couronne extraite du lion couronné est rapportée sur les lions dessinés tête
   nue (`crown_anchor`). L'aigle de l'Empire perd la couronne du dessin (pas avant 1433).
4. **Les écus de faction changent volontairement.** Le principe « identiques à l'octet » de DA1
   est abandonné au profit d'un test de non-régression **par blason**
   (`tools/tests/test_heraldic_charges.py`) : chaque écu témoin doit montrer les teintes que
   nomme son blason, dans une part minimale, et un lion dessiné doit garder ses traits
   intérieurs. Les écartelés de faction (Castille, Hainaut) passent par la grammaire v2, qui lit
   la teinte de chaque quartier (le Hainaut était faux en v1).
5. **Étendards EP5** : `banners.build_houses()` produit bannière, pennon et étendard de chaque
   maison (`game/assets/heraldry/banners/houses/`, livrée = deux premières teintes du blason).
   `BattleStandards` reçoit la maison du général de chaque camp. Le premier porte-étendard du
   général porte la bannière de sa maison, en taille royale. Le second garde l'étendard de
   l'armée (bannière royale, saint Georges, oriflamme du roi en personne). « Pas de quartier »
   remplace toujours le premier. Les unités nobles de sa retenue (`house_arms.unit_types` de
   `data/fx/battle_standards.json`) portent aussi ses armes. Le commun garde l'étoffe de la
   faction. Maison inconnue ou fichier absent : repli sur la faction. Le drapeau-repère de
   `battle_scene.gd` (`_banner_cloth`) suit la même règle.

Conséquences : 29 écus de faction, 51 de maison, 87 étoffes de faction et 153 de maison
régénérés (`cent-ans assets heraldry`, `cent-ans assets banners`). Planches :
`docs/img/da1b/`. Limites : le dauphin est le dessin « pâmé » (bouche ouverte) faute d'un
dauphin vif libre de droits. Le lion de saint Marc est rampant, pas « en moleca ». Les petits
châteaux de la bordure de Portugal et des lambels restent polygonaux (trop petits pour le
dessin). Les porte-étendards eux-mêmes gardent le surcot de faction.
