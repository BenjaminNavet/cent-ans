# RX — rapport `bataille-v2` (09/10/2026, après fusion TX, main eea65a370)

Méthode : 8 captures (`/private/tmp/claude-501/rx-shots/bataille-v2/` : contact, closeup, village, cav, siege, rain, snow, dawn), via `gpu_lock.sh` + `godot_bg.sh`.
Limite : load machine 50 puis 130 au moment de la sonde. **Sonde d'issue `--autoplay` non faite** (machine non calme, et aucune option de graine/sortie chiffrée n'existe dans `battle_scene.gd` ; reste à écrire côté outillage). `smoke_battle.gd` non relancé.

## 1. Verdict
- Forces : le sol de près (herbe, touffes, chaume sec) est beaucoup plus riche qu'avant TX ; le hameau à 60 m (pierre, colombage, fenêtres) est crédible et cohérent avec les soldats ; l'aube, la pluie et la neige changent vraiment l'ambiance sans casser la lisibilité des régiments.
- Faiblesses majeures : de haut/moyen plan le sol montre un motif de « confettis » (feuilles) à grande échelle, et en neige un tuilage en lattes très visible ; le gros plan `--closeup` par défaut est toujours dans un toit ; les échelles de siège sont inchangées (boîtes orange) ; les maisons de la ville assiégée sont orange uni, criblées de trous.

## 2. Suivi des constats du premier passage

| Constat v1 | État |
|---|---|
| smoke_battle bloqué en headless | non rejoué (machine saturée) : toujours à vérifier |
| Colonne unique sur le pont | toujours là (rain/snow : toute l'armée sur le pont à 3:37, même graine) ; non traité par TX |
| Seuil de déroute 40 % | non remesuré (pas de sonde) |
| `--closeup --shot-at` ne trouve pas la mêlée | changé : le cadrage `closeup_shot` (A1-06) existe ; mais sans `--shot-at`, la caméra à 26 m est dans le toit (voir nouveau 1) ; avec `--shot-at=100` la mêlée n'a pas eu lieu, champ vide (cav.png) : toujours inutilisable pour la cavalerie avant contact |
| Tuiles blanches fin de bataille, bandeau déploiement, avertissement `CampaignSim` | avertissement toujours là (log contact/closeup) ; les deux autres non revérifiés |
| assets3d : toit étiré en gros plan | **toujours là**, légèrement changé : rayures d'ardoise verticales plus nettes mais coutures horizontales et bandes toujours visibles (closeup.png) |
| assets3d : échelles orange | **toujours là** (siege.png) |
| assets3d : défenseur statique | non revu |
| assets3d : roseaux `dn/` cassé | non revérifié (hors bataille) |
| assets3d : poids textures | aggravé par TX (+81 Mo sols de bataille, 33 Mo bâtiments, 20 Mo eau) mais hi/ retirés du dépôt |
| shader `soft_distance` | aucune erreur dans les 8 journaux (grep négatif) : résolu |

## 3. Constats

### [majeur] [bug] Gros plan par défaut toujours à l'intérieur d'un toit
**Constat** : `--closeup` (26 m) donne un pan d'ardoises plein cadre, sans soldats ; à 60 m le même hameau est superbe. La texture de toit (bandes verticales + joints horizontaux nets) trahit le tuilage.
**Preuve** : `closeup.png` vs `village.png`, même graine (Île-de-France, 3:25).
**Correction proposée** : `battle_capture_stage.gd closeup_shot` : écarter le foyer ou relever/rapprocher la caméra si un bâtiment est entre caméra et foyer (raycast ou rayon d'exclusion autour des bâtiments) ; pour les joints, vérifier `tile_m` de la matière toit ardoise dans `data/art/textures/building_materials.yaml` (2,4 m) et la rotation d'UV sur pans inclinés.
**Coût** : M

### [majeur] [finition] Sol de bataille : motif de « feuilles » répétitif à moyenne distance
**Constat** : sur l'herbe, de la caméra par défaut, le sol est un patchwork de petites feuilles vertes contrastées (dawn.png, siege.png, rain.png), visible en pavage régulier ; à distance courte (cav.png) l'herbe est excellente. Le contraste du motif est plus élevé que celui des soldats, ce qui fatigue l'œil et fait perdre la lisibilité des silhouettes sombres sur sol sombre (pluie).
**Preuve** : `siege.png` (pente à gauche), `dawn.png`, `rain.png`.
**Correction proposée** : réduire le contraste et l'amplitude de la couche `micro_battle`/cartes au sol avec la distance (fondu plus tôt dans `battle_terrain` shader), augmenter la période du tuilage ou ajouter la rupture de tuilage par bruit basse fréquence (déjà utilisée sur la campagne).
**Coût** : S à M

### [majeur] [bug] Neige : sol en lattes parallèles
**Constat** : en `--weather=snow`, le sol apparaît comme un plancher de lattes régulières avec taches vert pâle ; aucune couche neigeuse réelle, le motif des textures sous voile gris devient une grille anisotrope. Les toits blanchissent bien.
**Preuve** : `snow.png`.
**Correction proposée** : vérifier le paramètre de neige du shader de sol (mélange vers blanc/bruit) : le sol doit blanchir avec bruit non périodique, pas seulement voiler ; contrôler l'anisotropie du filtre.
**Coût** : M

### [majeur] [finition] Siège : maisons de la ville orange uni, trouées
**Constat** : toutes les maisons de Bordeaux ont la même teinte orange saturée sans variation, façades presque sans texture, fenêtres comme trous blancs ; contraste fort avec murailles en pierre et tours réalistes. Les maquettes de ville n'ont pas reçu les textures TX (cf. « maquettes lointaines : atlas par défaut » dans `docs/wip/tx-veg-build.md`).
**Preuve** : `siege.png`.
**Correction proposée** : appliquer l'atlas régional (`building_atlas`) aux bâtiments intra-muros du siège, ou au moins désaturer et varier la teinte par instance.
**Coût** : M

### [majeur] [finition] Échelles de siège inchangées
**Constat** : boîtes orange, épaisses, détonnant avec murs, soldats et sol.
**Preuve** : `siege.png`, `battle_siege.gd:56`.
**Correction proposée** : matière bois de l'atlas bâtiments (grain, teinte vieillie), barreaux plus fins.
**Coût** : S

### [mineur] [finition] Toits coniques des tours : écorce
**Constat** : les toits des tours semblent recouverts d'une texture d'écorce/paille brune sans joint de bardeau lisible ; plausible mais mat ; cohérent entre elles.
**Preuve** : `siege.png`.
**Correction proposée** : matière ardoise ou tuile pour les tours des villes françaises.
**Coût** : S

### [mineur] [finition] Arbres en lisière : masse noire plate
**Constat** : le rideau d'arbres du fond reste une bande sombre homogène, la nouvelle écorce n'est pas visible à cette distance (cohérent) ; de près (cav.png) les troncs et la silhouette des feuillus sont corrects.
**Preuve** : `contact.png`, `cav.png`.
**Correction proposée** : éclaircir légèrement le feuillage lointain (brume) pour casser la masse noire.
**Coût** : S

### [mineur] [bug] Cadrage de cavalerie avant contact impossible
**Constat** : `--closeup --shot-at=100 --closeup-distance=14` cadre un champ vide (aucune unité ne s'est engagée). Pas de moyen de capturer une charge de près.
**Preuve** : `cav.png`.
**Correction proposée** : option `--follow=<id|type>` ou cadrage `--closeup` sur l'unité de cavalerie la plus proche de l'ennemi.
**Coût** : S

### [mineur] [finition] Avertissement `CampaignSim` hors campagne
**Constat** : toujours émis à chaque lancement (inchangé).
**Preuve** : `contact.log`, `closeup.log`.
**Correction proposée** : garde `has_campaign()` avant `get_character` (`living_portrait.gd:306`).
**Coût** : S

### [mineur] [conception] Armée entière sur le pont (rappel)
**Constat** : même scénario ; en pluie et neige la colonne est parfaitement lisible mais bloquée.
**Preuve** : `rain.png`, `snow.png`.
**Correction proposée** : voir rapport v1 (IA, aperçu de chemin).
**Coût** : M

## 4. Météo et aube (nouveau)
- Pluie : rendu cohérent (brume, sol assombri, gouttes, toits mouillés bleus) ; régiments lisibles ; l'eau est un noir pur, un peu opaque.
- Aube : très beau (lumière rasante, ciel orange, ombres longues) ; texte de bandeau « portée des tireurs −30 % » bien affiché.
- Village et ville en bataille : hameau de près excellent ; ville du siège voir constat 4.

## 5. À ne pas changer
- Herbe et chaume de près, écorces de feuillus de près, hameau en pierre/colombage, mêlées et étendards, siège (murs, tours, bannières, barre d'état), ambiance aube/pluie, aucune erreur shader dans les journaux.
