# 0156 — Lumière de la carte : SSIL gardé, SDFGI écarté, brume du matin dans le shader du sol

Date : 2026-10-02. Statut : acceptée. Lot TB6 (`docs/design/2026-10-02-campagne-tob.md` § 3).
Complète les ADR 0123 (budget d'images), 0142 (nappe de brume plafonnée), 0150 (saisons) et le
lot TB2 (nuées sobres).

## Contexte
TB6 demande un essai mesuré du SSIL, puis du SDFGI s'il reste de la marge, une brume du matin
basse dans les vallées, une lumière dorée par défaut, et la correction de grandes taches sombres
aux bords nets vues en vue moyenne et large alors que le temps paraît clair.

Mesures du 02/10 à 1600 × 900, préréglage Haute (MetalFX spatial 0,75, soit 1200 × 675 rendus),
Paris, sur une machine très chargée (charge moyenne 30 à 80, autres agents). Outil :
`game/tests/tb6_shot.gd --bench`. Un banc qui attend `process_frame` (`ss_shot.gd --bench`) est borné
par le compositeur quand la fenêtre passe en arrière-plan (6,90 ms et 16,68 ms relevés), et la
mesure GPU du moteur rend 0 sous Metal. Le banc TB6 rend donc en boucle serrée (`RenderingServer.force_draw`, sans
vsync ni plafond d'images), alterne les configurations sur 7 tours dans le même processus, et donne
l'écart apparié par tour (médiane) et l'écart des meilleurs tours.

| Effet (coût en ms par image) | d = 400, apparié | d = 400, meilleurs tours | d = 90, apparié | d = 90, meilleurs tours |
|---|---|---|---|---|
| SSIL (qualité basse, demi-définition) | 0,21 / 0,65 / 1,14 | 0,87 / −3,11 / 0,48 | 0,51 / 0,32 / 2,82 | 0,13 / 0,34 / 1,02 |
| SDFGI en plus du SSIL | 0,88 / 3,22 / 1,74 | 1,07 / 1,50 / 2,11 | 1,45 / 4,48 / 3,75 | 1,74 / 0,69 / 0,90 |
| Brume de vallée (shader du sol) | 0,44 / −0,05 / 2,05 | 0,62 / −3,11 / 0,55 | −0,21 / −0,61 / −0,83 | −0,21 / 0,21 / 0,09 |
| Brouillard volumétrique (grille 64 × 48) | 2,39 | 0,02 | 0,17 | 0,17 |

Trois lancements par case (un seul pour le volumétrique). Image entière : 13,5 à 21 ms dans les
tours calmes, jusqu'à 35 ms sous la charge. Le bruit d'un tour à l'autre (± 2 ms) dépasse le
seuil de 1 ms : ces chiffres sont indicatifs.

## Décision
- **SSIL : gardé tel quel.** Il était déjà actif sur la carte en Haute et en Ultra avant TB6 (clé
  `ssil` des préréglages de `RenderQuality`, coupé en Basse et Moyenne). Médiane des six écarts
  appariés : 0,58 ms ; quatre sur six sous 1 ms. Aucun changement de code ; il reste derrière le
  réglage de qualité existant.
- **SDFGI : écarté sur la carte.** Il coûte de 0,9 à 4,5 ms en plus, alors que l'image de la carte
  dépasse déjà le budget en Haute (ADR 0123). Le relief est déplacé dans le shader et les marqueurs
  bougent : le SDFGI n'aurait presque rien de statique à éclairer. `RenderQuality` le réserve à la
  bataille (inchangé).
- **Brume du matin : brouillard de hauteur calculé dans le shader du sol**, pas de brouillard
  volumétrique. La nappe apparaît là où le sol est plus bas que l'altitude moyenne alentour
  (niveau grossier de la carte des hauteurs, une lecture de texture), s'épaissit en vue rasante,
  se lève au fil du tour et s'efface sur la province sélectionnée (règle TB2). Coût dans le bruit
  de mesure. Le brouillard volumétrique de l'environnement est écarté pour sa fonction plus que
  pour son coût : global, il ne suit ni le masque météo par province ni le dégagement de la
  province sélectionnée, et sa grille est trop grossière aux distances de la carte (90 à 1100).
- **Ombres de nuages bornées.** Cause des taches : le masque météo du sol multipliait le sol
  mouillé (−28 %) par l'ombre des nuées (−35 %, bord de 0,22), par paliers d'un tiers au bord des
  provinces, alors que le plan de nuées est presque transparent en vue moyenne (opacité 0,11).
  Désormais sol mouillé −4 %, ombre des nuées −6 % large et fondue, ombres de nuages du terrain
  −5 % : au plus −14,3 % cumulés. Rien par temps clair (règle TB2 conservée).
- **Lumière dorée par défaut** dans `data/fx/atmosphere.json` (bloc `campaign`) : soleil à 20° /
  21° / 18° au printemps, en été, en automne (26 / 28 / 22 avant), plus chaud et plus fort pour
  garder la clarté du sol plat ; hiver inchangé (18°, soleil pâle, étalonnage froid de TB1). La
  bande 18-28° de PO3 est respectée.

## Conséquences
- Valeurs dans `data/ui/campaign_map.json` (blocs `clouds` et `morning_mist`) et
  `data/fx/atmosphere.json` ; aucune règle de jeu, batailles inchangées (le bloc `campaign` et le
  shader du terrain de carte ne servent qu'à la carte).
- La nappe est peinte sur le sol : arbres, villes et figurines en émergent sans être voilés.
- À refaire sur une machine calme : `tb6_shot.gd --bench` (SSIL à confirmer sous 1 ms), et le
  jugement à l'œil de la lumière dorée et de la brume (session principale).
- Le voile de parchemin du brouillard de guerre (TB2, `fog_dim` 0,72) assombrit les provinces hors de vue
  (jusqu'à −35 % mesurés à 400, 9 % du sol vu depuis Paris), avec un bord net : hors de ce lot, signalé à la session principale.
