# RX campagne-v2 — re-jugement de la carte après TX (main eea65a370)

Captures (6) : `/private/tmp/claude-501/rx-shots/campagne-v2/{france,normandie,paris_close,parchemin,mediterranee,oural}.png`.
Cadrages reconstruits (le premier rapport ne notait pas les coordonnées) : France `2213,3300,900`, Normandie `2050,3200,250`, Paris `2213,3204,22`, parchemin `3000,3300,2000`, Méditerranée `2400,4050,250`, Oural `6500,1300,900`. Les vues sont donc proches mais pas identiques à celles de `campagne.md` ; l'Oural en particulier tombe sur un autre secteur (bord de mer Caspienne/Noire à droite). 1337 printemps, mêmes nuages.
Limites : pas de test de zoom intermédiaire, pas de comparaison `--legacy-textures` côte à côte, tuilage jugé à l'œil sur 6 vues.

## 1. Verdict
- TX ne change presque pas l'image à l'échelle région : la couleur reste dictée par la colormap (cohérent avec `tx-campaign.md` « le fond régional change peu l'image »). Aucun constat du premier rapport n'est résolu.
- Gain net : relief du sol un peu plus granuleux de près (Paris, Normandie) ; Camargue et marais mieux texturés.
- Nouveau défaut au dézoom (France, Oural) : le sol ocre/brun se répartit en grandes taches « camouflage » sombres/claires, avec des motifs répétés visibles.
- Les maquettes blanches et les dalles blanches autour de Paris sont strictement inchangées : c'est toujours le défaut n°1 de la carte.

## 2. Suivi des constats de `campagne.md`
| Constat initial | État | Preuve |
|---|---|---|
| Maquettes blanches sur-exposées | **Toujours là**, identique | normandie.png (Caen, Le Mans, Évreux), mediterranee.png (Montpellier, Aix, Toulon) |
| Dalles blanches froissées autour de Paris | **Toujours là**, identique | paris_close.png (des dizaines de dalles crème en tous plans, y compris sur champs) |
| Nuages trop opaques au tour 1 | **Toujours là** | france.png (sud, Limoges/Lyon masqués), oural.png |
| Forêts sombres / plaines ocre en deux aplats | **Changé, pas résolu** : l'ocre est un peu plus sombre et tacheté, mais toujours saturé et sans parcelles lisibles | france.png, normandie.png |
| Rouge anglais saturé sur le sol | **Toujours là** | france.png (Angleterre), normandie.png (haut) |
| Étiquettes peu contrastées sur forêt | **Toujours là** | normandie.png (Domfront, Sées, Villedieu) |
| Parchemin : fleuves trop épais/sombres, semis d'arbres | **Toujours là, pire à l'œil** : rubans bleu nuit épais couvrant les frontières | parchemin.png (Rhin, Danube, Loire) |
| Méditerranée : mer vide, Camargue en aplats | **Partiellement amélioré** : marais de Camargue et étang de Berre texturés ; mer toujours vide | mediterranee.png |
| Armées / écus à d=900 | Inchangé, correct | france.png |

## 3. Nouveaux constats (dus à TX)

### [majeur] [finition] Taches « camouflage » à grande échelle sur le sol ocre
**Constat** : en vue France, le Bassin parisien et le Centre-Ouest (Poitiers, Bourges) montrent des cellules sombres/claires de 20-60 px, nettes, qui se répètent (aspect peau de léopard / camouflage), surtout là où aucune forêt ne les masque. Sur l'Oural (sol brun), même motif en plus sombre. Ça évoque un bruit de micro-grain ou de mélange régional échantillonné trop gros au dézoom, sans fondu de mip.
**Preuve** : france.png (zone Tours-Bourges-Poitiers), oural.png (tout le sol), normandie.png (plaine Sud-Est).
**Correction proposée** : dans les données `micro`/`regional` de `campaign_terrain_textures.json`, réduire l'amplitude du micro-grain avec la distance caméra (fondu vers 0 au-delà d'environ d=400) et vérifier le LOD/mipmap des tableaux ; sinon baisser `hue_keep`.
**Coût** : S-M

### [majeur] [finition] Oural : sol brun terne, grandes zones homogènes, bord de biome rectiligne
**Constat** : le sol de l'Oural est un brun-gris peu lisible (peu de parcelles, forêts mal distinguées du fond, ombres de nuages grises mêlées au sol) ; en haut à droite un rectangle vert au contour parfaitement droit (rupture de biome en escalier) et, côté mer, une bande sombre au bord franc.
**Preuve** : oural.png (haut droite).
**Correction proposée** : vérifier `geo biome-blend` sur ce secteur (le fondu de 10 km ne s'applique pas à ce bord, ou la carte de biomes est en basse résolution) ; ajouter de la variation de teinte par rôle pour l'Est.
**Coût** : M

### [mineur] [finition] Cohérence Paris de près : sol olive plat, triangles sombres de relief
**Constat** : de près le sol est un olive uniforme avec de grandes facettes triangulaires sombres sur les collines (relief), peu de texture de champs visible autour de la ville malgré les packs ; l'écart avec les toits très détaillés de Paris est fort. Les « champs » restent sous forme de dalles blanches (cf. constat toujours ouvert).
**Preuve** : paris_close.png (gauche et haut).
**Correction proposée** : confirmer que `parcels_source=tx` est actif dans cette vue (le défaut reste `hb` selon `tx-campaign.md`) ; trancher après la planche `fields_compare.png`.
**Coût** : S

### [mineur] [finition] Transition entre niveaux de zoom : teinte et grain changent
**Constat** : entre France (d=900) et Normandie (d=250), l'ocre passe d'un orange-jaune tacheté à un jaune-olive plus doux avec davantage de vert ; la forêt passe de plages sombres à des houppiers individuels. Pas de saut brutal, mais la couleur du même lieu (Orléanais, Beauce) diffère nettement d'un niveau à l'autre.
**Preuve** : france.png vs normandie.png (Normandie est / Paris).
**Correction proposée** : harmoniser la saturation de `colormap_style.yaml` avec celle du pack régional ; mesurer par pixel moyen sur une zone fixe à trois distances.
**Coût** : M

### [mineur] [finition] Coutures de textures et tuilage
**Constat** : aucune couture évidente ni répétition d'une tuile unique de forêt ou de champ n'a été vue sur les 6 vues ; le seul tuilage visible est le motif tacheté ci-dessus. Pas de couture entre biomes autre que le bord rectiligne de l'Oural. À rejuger de près en plaine (d < 25), non couvert par la capture Paris.
**Preuve** : toutes les captures.
**Correction proposée** : une capture de contrôle en plaine à d=40 sur Beauce.
**Coût** : S

## 4. À ne pas changer
- Forêts généralisées et lacs éclaircis ; relief ; brouillard plafonné.
- Parchemin : ornements, frontières par posture, lisibilité globale (hors fleuves).
- Paris 1:1 (cathédrale, toits, Seine).
- Plaques d'armée et écus de ville.
- Fondu de frontière de biome sur la France et la Méditerranée : aucune rupture visible.
