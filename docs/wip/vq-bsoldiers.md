# VQ bsoldiers — soldats de bataille réalistes (17/10)

## État
Fait ; clippy -D warnings propre, 53 tests units+quality verts (release). Lot : matières (acier/mailles, étoffes moins saturées, peau), distance de bascule imposteur (`battle_lod`), variation par soldat.

## Fait
- `data/fx/battle_soldier_look.json` (nouveau), lu par `looks::SoldierLook` et versé dans 3 `vec4` du bloc de paramètres (`P_LOOK`, `look0..2` de `skinned.wesl`, bloc 131 -> 134 vec4) :
  - livrées : désaturation 0,3 -> 0,5, valeur 0,62-0,9 -> 0,5-0,78, étoffes unies x1,6 -> x1,35, lueur lointaine 0,35 -> 0,1 (c'était le « bleu jouet » à 70-175 m) ;
  - variation par soldat : lavage (part de gris) et dérive chaud/froid de la livrée, grain de tissage + crasse du bas (étoffes, fondus avant 60 m) ;
  - boucliers peints (`C_ARMS`) : peinture écaillée aux bords (bois nu), rayures, terre remontant du sol, taches (fondu 30-120 m) ;
  - acier/mailles : albedo mélangé à un gris froid x0,9 (plus sombre, lit comme du métal) ; peau : retour vers un ton neutre (0,5) au lieu de l'orangé (aussi dans le chemin GA3) ;
  - métal : metallic/roughness déjà bons (plaque 0,85/0,28-0,5, mailles 0,8/0,6) et occlusion spéculaire du ciel (CR2/CR4) déjà en place : non retouchés.
- `battle_lod` du préréglage lu par `RenderQuality.battle_lod` : multiplie le bord imposteur (300 m) de `Lod::for_distance_scaled` ; `high` 1,0 -> 1,5 (450 m), `ultra` 1,3 -> 2,0 (600 m) ; low 0,55 / medium 0,75 (désormais effectifs, bord à 165 / 225 m). Surcharge de dev `CENT_ANS_BATTLE_LOD=<facteur>`.

## Mesures
- Dump à l'ouverture de Crécy (déterministe) : soldats en figures (mesh lointain) 1440 (x1,0) -> 1640 (x1,25) -> 1720 (x1,5 = x2,0 : le reste est au-delà de 600 m) ; imposteurs 1880 -> 1600. +19 % de figures lointaines, mesh à faible nombre de triangles.
- A/B alterné `ab.sh battle` (3 paires, charge machine ~135 : les A seuls varient de 2x d'une série à l'autre, donc bruit ; deltas médiane/p95) : x1,5 : calm +2,2/+6,3 ms, melee +10,6/+2,1 ; x2,0 : calm -14,9/-12,6, melee +20,5/+17,3. Signes contradictoires en calm, melee (caméra à 70 m, bord imposteur sans effet) : bruit pur. À remesurer machine au calme (porte ADR 0404 : melee p95 <= 25 ms) ; si dépassement, baisser `battle_lod` dans `render_quality.json` (données seules).
- Coût des matières : branches ajoutées = bruit de valeur + quelques smoothstep, pas de texture ; non mesuré isolément (pas d'interrupteur sans reconstruire 23 min ; racine alternative `CENT_ANS_ROOT` avec grain/usure à 0 prévue mais non lancée).

## Décisions (pas d'ADR numéroté disponible)
- Réglages de finition en données (`battle_soldier_look.json`), défauts Rust identiques (test de concordance).
- On n'a pas touché aux imposteurs (ombres I009, sang/casque I011) ni aux animations ; l'imposteur reprend seulement la teinte de livrée `dyed_cloth` (couleur, pas de forme).
- Les valeurs de `battle_lod` sont ajustables sans reconstruire.

## Reste
- Regard du joueur sur le métal (reflets du ciel faibles sur plaque en contre-jour), cottes de mailles de près (non vues : captures budget 6).
- Mesure A/B au calme ; mesure isolée des matières.
- Variation par soldat pour les chevaux (robes déjà variées) non touchée.
