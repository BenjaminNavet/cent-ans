# DN — Proposition de révision de la bible DA (non appliquée)

Sources : bible, pipeline, `materials.yaml`, `ga3_decor.json`, ADR 0004/0124/0136/0138/0144/0158/0161/0164/0190/0210. Rien lancé.

## 1. Contradictions et zones floues
1. **Maquettes stylisées (ADR 0158) contre « une seule main » (pilier 3) et § 6.** La bible vise le
   réalisme peint à toute échelle ; 0158 retient des maquettes « peu d'éléments, gros et clairs » et
   un grossissement constant. Aucun texte ne dit que c'est un *registre voulu* (généralisation
   cartographique) ni ce qui reste semi-réaliste dedans (matière, lumière, palette).
2. **§ 7 périmé.** Il cite « Carte, 1-5 m (ZG) : villes à l'échelle, ponts » et un « langage de
   marqueurs de ville » : 0138 puis 0158 ont supprimé la vue 1:1 agrandie et 0124 les pictogrammes.
   § 8 « marqueurs de carte : forme = type » contredit 0124 (« aucun pictogramme de ville »).
   § 10 lot DA3 (marqueurs disparates) est donc sans objet.
3. **Deux vues (0124).** La vue stratégique (parchemin) est un troisième registre non prévu par
   § 1 (« deux registres seulement ») : ni enluminure d'interface, ni réalisme peint. À rattacher
   explicitement à l'enluminure (carte-objet) pour garder la règle des deux registres.
4. **Figurines générées contre ADR 0136 § 3.** 0136 : « Pas de soldats générés en 3D (TRELLIS) :
   squelette et animations l'interdisent ». 0210 et le pipeline adoptent TRELLIS/SF3D pour les
   figurines sans dire comment elles reçoivent squelette, animations, canaux de livrée. Les
   budgets § 6 (12 000 / 1 350 / 260 triangles, FG) supposent des figurines maison : un glb TRELLIS
   brut fait 100 k+ triangles et une texture cuite sans canal de livrée.
5. **Livrée « vert pur » (pipeline § 2) contre § 3.1 et § 6.** Le procédé demande vert/bleu purs
   saturés comme canaux de recoloration ; la bible dit « albédo tel quel » des teintes héraldiques.
   Un glb texturé cuit n'est recolorable que par masque de teinte : règle à écrire.
6. **Palette § 3.3 trop vague.** « Saturation ≤ 35 % » sans valeur de clarté ni d'albédo ; or
   `ga3_decor.json` demande « bright natural colours » (contraire au pilier 1) et un fond gris
   neutre sans contrôle d'exposition ; `cleanup` retouche à la main (`auto_levels`, `gamma`) faute de règle.
7. **Budgets triangles et LOD** : seuls les figurines ont un budget ; rien pour décor, bâti,
   machines de siège, végétation générés (ga3 : `lod0` 8 000 pour une maison, sans règle commune).
8. **Divers** : § 6 cite Poly Haven (on utilise ambientCG) ; § 5 un seul modèle par défaut (Z-Image local, 0190) ; § 10 dette lots DA1-DA5 caducs ou faits. **Pas de règle sur échelle, orientation, pivot, nommage** des glb : chaque lot (ga3 `size_m`,
    `scale_axis`, `fit`) a ses conventions.

## 2. Règles révisées, prêtes à coller

### § 1 bis — Trois états du réalisme peint (à ajouter après la table des registres)

Le réalisme peint a trois états selon la hauteur de caméra ; ils partagent matières, lumière et
palette, et ne diffèrent que par le **niveau de généralisation** :
**détaillé** (bataille, siège, rig de campagne proche) ; **généralisé** (carte de campagne à hauteur
de jeu, ADR 0158 : maquettes, arbres, champs à taille monde constante) ; **cartographique** (vue
stratégique, parchemin CM2, ADR 0124), qui relève du registre enluminure (carte-objet à l'encre).
Aucun pictogramme de ville.

### § 3.3 — Monde 3D : palette (remplace le texte actuel)

Mesures en HSV, sur l'albédo et sur le rendu final en plein jour (hors livrées) :

| Famille | Saturation | Valeur (clarté) | Teintes de référence |
|---|---|---|---|
| Terre, chemins, pierre calcaire | 0,10-0,30 | 0,35-0,60 | ocre gris, beige, brun-gris |
| Herbe, forêts (été) | 0,22-0,35 | 0,25-0,45 | vert olive, jamais vert vif ; teinte 70°-105° |
| Bois, chaume, toits de tuiles | 0,15-0,35 | 0,18-0,40 | brun 20°-35° ; chaume gris-brun sombre |
| Laine, lin, cuir (commun) | 0,05-0,30 | 0,25-0,70 | écru, brun, gris, bleu et vert passés |
| Acier, mailles, plates | 0,00-0,08 | 0,35-0,60 | gris-bleu sourd, jamais miroir |
| Eau | 0,15-0,35 | 0,20-0,50 | prend le ciel ; pas de turquoise |
| **Livrées, bannières, écus** (§ 3.1) | **0,60-1,00** | 0,25-0,95 | teintes héraldiques exactes |

Règles : (a) aucun albédo hors livrée au-dessus de S 0,40 ; (b) albédo moyen d'un décor
0,18-0,35 (jamais blanc pur ni noir pur : plancher 0,03, plafond 0,85, repère neige) ;
(c) la saturation des livrées reste hors de ces plages : c'est ce contraste qui fait lire les camps ;
(d) saisons : déplacent la teinte, pas la saturation (inchangé) ; (e) les ombres sont froides
(ciel), les lumières chaudes ; contraste d'ensemble faible, pas de noirs écrasés ; (f) test de
conformité : histogramme S du rendu, 95 % des pixels hors livrée sous 0,40.

### § 6 — Réalisme peint (remplace le texte actuel)
- **Matières.** PBR : albédo + normale + rugosité ; sources : ambientCG CC0 (`materials.yaml`),
  CC0/domaine public FA (ADR 0164), textures procédurales maison. Les matières générées (peau,
  cheveux) restent GA1. Quaternius/Kenney retexturés seulement.
- **Rugosité (cible après étalonnage).** Laine, gambison, toile 0,80-0,90 ; lin 0,75-0,85 ;
  cuir ciré 0,50-0,65 ; bois vieilli 0,70-0,80 ; pierre 0,75-0,90 ; mailles 0,35-0,50 ; plates
  0,30-0,45 (jamais < 0,25 : pas de chrome) ; métal rouillé 0,60-0,80. Métallique : 0 sauf métal nu (1).
- **Usure obligatoire** (ADR 0136) : boue au pied jusqu'à ~0,3 m (figurines) ou ~1 m (bâti), crasse des
  creux, liseré d'usure sur les arêtes, teintures passées (livrées à ±6 % de luminosité, jamais
  neuves), toits et chaume patinés. Un asset neuf et propre est refusé.
- **Échelle des matières.** Un motif se lit à sa taille réelle : trame de laine ~2 mm, anneau de
  maille ~9 mm, planche ~15-25 cm, tuile ~20 cm. `tile_m` (physique) fait foi, pas le pixel.
- **Densité de détail.** Un seul niveau de détail secondaire par palier ; pas de bruit haute fréquence qui scintille (≥ 4 px par motif à la distance d'usage).
  qui disparaît ou scintille au mouvement (normales à ≥ 4 px par motif à la distance d'usage).
- **Silhouettes.** Chaque type d'unité se reconnaît en silhouette noire à 40 px de haut :
  coiffe/casque, arme d'hast, écu, monture. Arbalétrier ≠ archer ≠ homme d'armes ≠ coutilier.
- **Interdits.** Chrome et reflets miroir ; émission hors feu ; couleurs néon ; textures
  photographiées avec ombre ou perspective incrustées ; anachronismes (§ 2 pilier 2) ; brique
  et verre industriels ; personnages au visage « photo moderne » ; bras en croix figés dans le jeu
  (la pose en T de génération disparaît au rig) ; texte lisible sur un objet.
- **Figurines** : budgets inchangés (12 000 / 1 350 / 260 ; monté 17 500 / 2 100 / 550), squelette partagé,
  livrée par masque. Une figurine issue du procédé image → 3D n'est admise que **retopologuée et
  rigguée** sur le squelette partagé (voir § Assets générés) ; sinon statique seulement.
- **Lumière** : inchangé, plus : exposition cible 18 % de gris moyen sur le sol, soleil 15-35°,
  brume bleu-or (jamais grise), AgX, pas de bloom visible hors feu et soleil bas.

### Palier par palier : semi-réaliste ou généralisé (remplace § 7 « Porteur »)

| Palier | État | Semi-réaliste (obligatoire) | Généralisé (admis) |
|---|---|---|---|
| Bataille, rapproché (< 12 m) | détaillé | figurines, armes, matières, usure, armoiries, herbe, arbres réels | rien : pas de forme jouet |
| Bataille, haut | détaillé | matières et lumière ; drapeaux d'unité | figurines en LOD2/imposteurs ; végétation en cartes |
| Campagne, rig proche (< 45 u) | généralisé | matières, lumière, relief, teinte des toits | maquettes stylisées de taille monde constante (0158), arbres généralisés (0161) |
| Campagne, hauteur de jeu | généralisé | relief, eaux, forêts en volume, bannières d'armée saturées | maquettes lisibles (silhouette : enceinte, clocher, tours), champs à taille constante |
| Campagne, > 1200 | cartographique | encre, parchemin, vignettes de villes | tout (carte-objet) ; aucun pictogramme |
| UI, portraits, événements, codex | enluminure | tempera, or, ancre § 13 | aplats d'or et d'azur |

Règle : une maquette stylisée reste **semi-réaliste en matière** (albédo sourd, rugosité, usure de
toit, ombre portée) ; seule sa *forme* est simplifiée. Interdit : aplats saturés, contours noirs
cartoon, couleurs de toit hors du § 3.3.

### Nouveau § 14 — Assets générés (image → 3D, ADR 0210)
Prompts de style : voir § 3 ci-dessous ; jamais l'ancre `anchor.jpg`. Correctif § 8 : supprimer « marqueurs de carte : forme = type » (0124, aucun pictogramme de ville) ; § 6 « Poly Haven » → ambientCG.

**14.2 Acceptation d'une image** (avant toute 3D) : objet entier, centré, 85-90 % du cadre ; fond uni détourable proprement ; aucune ombre au sol ni sol ; aucun texte ; époque tenue (1337-1453) ; S moyenne hors livrée ≤ 0,35 et luminosité moyenne 0,25-0,60 (pas de HDR blanc) ; matière lisible (bois, pierre, chaume reconnaissables) ; pas de symétrie parfaite artificielle ; usure visible ; pas de pièces fusionnées ni de membres en surnombre ; figurines : bras horizontaux, mains ouvertes et vides, rien devant le corps, livrée en canal de recoloration. Meilleur-de-N (N ≥ 3) pour les modèles gratuits. 

**14.3 Acceptation d'un glb** : maillage étanche, sans trous ni faces internes flottantes ; pas de fragments détachés > 1 % du volume ; texture sans couture visible ni ombre cuite dure ; normales cohérentes (pas d'inversion) ; rendu de contrôle `ga3_compare_render.py` sous 4 angles, sans déformation de silhouette par rapport à l'image ; albédo moyen 0,18-0,35 (corriger par `auto_levels` + gamma documentés, jamais à la main) ; pas de lumière cuite (ombre portée, reflet). 

**14.4 Budgets triangles (LOD0 / LOD1 / LOD2) et textures**

| Classe | LOD0 | LOD1 | LOD2 | Texture |
|---|---|---|---|---|
| Accessoire (tonneau, caisse, étal) | 1 500 | 500 | 150 | 512² |
| Maison, grange, moulin, puits | 8 000 | 2 500 | 600 | 1024² |
| Bâtiment majeur (église, halle, tour) | 15 000 | 4 000 | 1 000 | 2048² |
| Machine de siège, charrette, bateau | 12 000 | 3 500 | 800 | 1024² |
| Arbre/arbuste (bataille) | 6 000 | 1 200 | carte | 1024² (feuilles en atlas) |
| Figurine (rigguée) | 12 000 | 1 350 | 260 | atlas du kit (inchangé) |

Décimation : sortie TRELLIS (≈ 100-300 k) → Blender `ga3_cleanup.py` : nettoyage, décimation
quadrique en conservant UV et bords, LOD1/LOD2 par ratio 0,3 / 0,08 ; vérifier silhouette à chaque
palier (écart de contour < 3 % de la hauteur). Plafond du lot = `lod0` du catalogue, jamais plus.
Texture : JPG/WebP compressé VRAM, 1 seule carte albédo + normale dérivée (rugosité constante par
matériau, tirée du § 6) ; pas de métallique sauf fer nu.

**14.5 Repère, échelle, orientation** : unités **mètres** ; +Y haut, −Z avant (convention glTF/Godot) ;
**pivot au sol** au centre de l'emprise (y = 0 au point le plus bas) ; l'objet fait sa taille
réelle (`size_m` du catalogue, axe `scale_axis`) ; figurine : 1,75 m ± 0,05, regard vers −Z ; la
sortie SF3D tournée de 180° en lacet est corrigée à l'import ; échelle appliquée aux sommets,
pas au nœud (scale = 1).

**14.6 Nommage** : `game/assets/models/<famille>/<id>[_lod1|_lod2].glb`, `id` snake_case anglais ; entrée `manifest.json` (emprise, triangles, modèles, graine) ; brutes hors dépôt ; `SOURCE.md` pour tout tiers.

**14.7 Figurines générées** : TRELLIS/SF3D produit une pose en T statique : usage direct limité aux
décors figés (cadavres, statues, personnages de scène). Pour le jeu animé : retopologie vers le
squelette partagé + transfert de poids, ou projection de la texture sur le corps du kit
(décision D2 ci-dessous). Livrée : masque de teinte (le vert pur sert de masque, puis albédo
neutre), recolorée par § 3.1.

## 3. Style canonique révisé

### `data/art/ga3_decor.json` (décor, Z-Image Turbo)
```
"style_prefix": "Photorealistic matte-painted reference photograph of a single",
"style_suffix": "Rural medieval France circa 1340, humble, heavily weathered and slightly irregular,
 hand-made natural materials (oak, lime plaster, fieldstone, thatch, hand-forged iron), real material
 scale and proportions, visible wear, moss, grime and mud at the base, no brick, no glass, no sheet
 metal, no modern elements, no text, no people, no animals. Muted earthy colour palette, low
 saturation, natural desaturated tones (greys, browns, ochres, olive greens), albedo like an
 overcast day, no neon, no bright primary colours. Three-quarter view from slightly above, the whole
 object fully visible, centred and filling about 85 percent of the frame, on a plain uniform medium
 grey studio background, no ground, no grass, no plants, no terrain, no base, no cast shadow on the
 floor, soft even diffuse overcast light, no baked directional shadow, no reflections, sharp
 realistic material detail, physically based look."
```
Garder `objects[].prompt` sans adjectif de couleur vive ; ajouter `target_albedo_mean` (0,18-0,35) au bloc `cleanup`.
### Équivalent figurines (Qwen-Image-Edit-2511, image de référence validée en entrée 1)
```
"fig_style_prefix": "Photorealistic reference sheet of a single",
"fig_style_suffix": "14th-century (1337-1453) European soldier, historically accurate period armour
 and clothing, plausible real-world proportions (1.75 m, natural stance, no heroic exaggeration),
 hand-made wool, linen, quilted gambeson, boiled leather, riveted mail and hand-forged steel with
 visible wear, mud at the boots, scratched and dulled metal, matte finish. Muted earthy palette for all
 clothing and gear except the livery: surcoat/jupon in pure saturated green (recolour mask),
 hose in pure saturated blue (recolour mask). Arms raised almost horizontal like Christ the
 Redeemer, palms open and empty, nothing in the hands or in front of the body, mounted figures
 with hands off the reins. Full body front view, three-quarter front, feet included, centred,
 filling 90 percent of a square frame, plain pure white background, no ground, no shadow, soft
 even diffuse light, no text, no heraldic emblem, no modern elements, no fantasy, no weapons."
```
Édition (Qwen) : « Keep identity, proportions and equipment of image 1; change only <pose|vue|
équipement>. » Le prompt négatif équivalent est le « no … » ci-dessus .

## 4. Arbitrages (D1-D5)
D1 maquettes 0158 = état généralisé du réalisme (proposé) ou 3e registre ; D2 figurines générées : retopo+rig / statique seul (0136) / projection sur le corps du kit ; D3 budgets 14.4 et textures 2048² ; D4 parchemin rattaché à l'enluminure, DA3 abandonné ; D5 plafond S 0,40 testé automatiquement ou simple repère.
