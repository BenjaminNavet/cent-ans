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
- Mesure de perf : voir `docs/wip/da1-heraldique.md`.
