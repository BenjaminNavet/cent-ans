# MX — Meshy : compte rendu de reprise (13/10)

Le joueur s'est abonné à Meshy (20 €/mois). Les assets prioritaires passent de TRELLIS 1 à Meshy, en
commençant par ce qui est vu de près. **Les images restent gratuites** : Z-Image Turbo local (`--local`,
mflux, ADR 0190) pour le décor, Qwen-Image-Edit local pour les retouches et vues de figurines. Meshy ne
sert qu'à l'étape image → 3D.

## Reprendre ici
**Règle du joueur (13/10) : à partir du mois 2 (crédits remis le 13/11), tout en meshy-6-lite, figurines
comprises** (15 crédits, ≈ 70 objets/mois). Mois 1 : 52 modèles (16 figurines 7.1, 33 décor 6-lite,
3 du banc), arrêt à 25 crédits avant siege_bombard_great_1450. Reprise : fin de la vague 4, vagues 5-7.
Outil : `python3 tools/experiments/meshy_waves.py run|status|sheets|ingest`.

1. Le MCP `meshy` est installé portée utilisateur (`~/.claude.json`, hors dépôt, connecté le 13/10) :
   ses outils `mcp__meshy__*` sont disponibles dans la nouvelle session. **Clé jamais dans le dépôt**
   (crochet local `pre-commit` du dépôt commun : refuse tout `msy_…` indexé).
2. `meshy_check_balance` avant et après chaque vague (13/10 : 1 060 crédits/mois).
3. **Vague 0 — mesurer le barème** (3 crédits d'essai) : un même objet (castle_byz) en `meshy-7.1`,
   `meshy-6`, `meshy-6-lite` texturés → écart de solde de chacun + planche. Le site annonce « 100 assets
   par mois » ; le barème API donne 30 crédits (meshy-6/7.1 texturé, ≈ 35/mois) ou 15 (6-lite, ≈ 70/mois).
   Choisir le modèle par catégorie selon la planche.
4. Ingérer d'abord les 4 générations déjà payées du banc (gratuit, voir § Banc).
5. Enchaîner les vagues de la liste ci-dessous jusqu'à épuisement des crédits du mois ; le reste au mois
   suivant (crédits remis à zéro, non reportés).

## Règles de génération
- **Décor : vue de face seule** (essai du joueur sur une abbaye byzantine : le dos est fidèle). Pas de
  vue de dos ni de profil à générer.
- **Figurines : planche 3 vues** face/profil/dos (format recommandé par Meshy, déjà le nôtre :
  `~/dev/cent-ans-raw/galerie-3d/1-figurines/<id>/<id>__image-planche-nb2-*.png`). Essai du joueur sur le
  coutilier (site web) : nettement meilleur que T1 (visage, rivets, maille, ceinture, bourse, dague).
  Poser `pose_mode: a-pose` et vérifier que le rig Quaternius suit (bras bas sur nos planches).
- **Image source** : variante `__r3d` (style « rendu 3D », règle T2) si elle existe, sinon la coupe
  `cut/s1337.png` du dossier DN. **Contrôler que la référence montre bien l'objet livré** : celles de
  `bombard` (chariot à boulets) et `siege_trebuchet_counterweight` (portique sans bras) ne correspondent
  pas → refaire l'image en Z-Image local avant d'appeler Meshy.
- `target_polycount` au budget de triangles de la classe (sinon 30 000 puis `dn-ingest` décime) ;
  textures 2K (la chaîne réduit).
- Téléchargement : `meshy_download_model` (GLB) vers `~/dev/cent-ans-raw/meshy/<id>/meshy.glb`, puis
  `cent-ans dn-ingest` (décor, `docs/pipeline-assets-3d.md` § 6) ou la chaîne figurines (§ Figurines de
  bataille). Ajouter le glb à la galerie (`object__image__meshy-<modèle>`).
- **Pas de retexture Meshy** : sur le banc, aucun gain visible sur T1 (texture juste plus bruitée).
- Rig et animations Meshy : non (squelette différent ; `docs/animation.md` : animations locales).
- Plugin Blender Meshy : inutile (pont interactif, nos scripts Blender sont headless).
- Licence : plan payant → usage commercial sans attribution ; ne pas utiliser de sortie du plan gratuit
  (CC BY 4.0).

## Liste prioritaire (ordre des vagues)
| Vague | Catégorie | Nb | Objets |
|---|---|---|---|
| 1 | Figurines (multi-vues) | 16 | coutilier, crossbowman, gascon_crossbowman, gendarme, knight, longbowman, man_at_arms, militia, plain_crossbowman, retinue, routier, schiltron, sergeant, standard_bearer, urban_militia, welsh_spearman |
| 2 | Châteaux | 15 | castle_balt, castle_byz, castle_byz_serbia, castle_empire_rhine, castle_hungpol, castle_iber, castle_isl, castle_isl_anatolia, castle_isl_andalus, castle_isl_mamluk, castle_ital_rocca, castle_med, castle_nord, castle_rus, castle_steppe (castle_west : déjà fait au banc) |
| 3 | Villes (maquettes) | 14 | city_balt, city_byz, city_hansa, city_hungpol, city_iber_mudejar, city_isl, city_isl_andalus, city_isl_mamluk, city_ital_commune, city_ital_maritime, city_med, city_nord, city_rus, city_steppe (city_west : banc) |
| 4 | Engins de siège | 15 | siege_tower_belfry_hides, siege_ram_roofed, siege_ballista_crossbow_great, siege_springald_bolt, siege_bombard_great_1450, siege_bombard_hooped_1380, siege_cannon_early_1340, siege_gun_bronze_wheeled_1450, siege_ribauldequin_organ, siege_gun_battery_mantlet, siege_powder_barrels_cart, wagon_ammunition_bombard, bombard (image à refaire), siege_trebuchet_counterweight (image à refaire, le banc a copié le mauvais portique), siege_handgonne_pair |
| 5 | Abbayes et églises | 15 | abbey_balt, abbey_byz, abbey_byz_serbia, abbey_iber, abbey_isl_anatolia, abbey_isl_maghreb, abbey_ital, abbey_nord, abbey_rus, abbey_steppe, abbey_ural_shrine, abbey_west, church_stave, church_wood_polish, fort_fortified_church |
| 6 | Fortifications | 8 | fort_barbican_gatehouse, fort_castle_keep_square, fort_shell_keep_motte, fort_palisade_corner_tower, fort_portcullis_gate, fort_drawbridge_lowered, ruin_castle, fort_stone_curtain_ruined |
| 7 | Ponts, portes, tours | 11 | bridge_gate, bridge_stone, bridge_stone_arch, bridge_wood, econ_town_gate_brick_hanse, econ_town_gate_horseshoe_bab, econ_town_gate_mediterranean, econ_town_gate_timber_rus, econ_town_gate_twin_towers, round_tower, watchtower |

Total ≈ 94 objets : ≈ 1 mois si les « 100 assets » du site valent pour l'API, ≈ 2-3 mois au barème
API pleine qualité. Ensuite : objets proches de la caméra en bataille (camp, chariots, faune). Le petit
décor lointain (≈ 600 objets) reste en T1.

## Banc fal (13/10, enveloppe 8 $, dépensé 6,90 $ — `docs/budget.md` § Banc Meshy MX)
`tools/experiments/meshy_bench.py run|sheets` ; sorties `~/dev/cent-ans-raw/meshy/<id>/`, planches
`~/dev/cent-ans-raw/meshy/sheets/<id>.jpg` (référence / T1 livré / T1 + retexture / Meshy v7.1).
- **Génération v7.1** : château nettement plus fidèle que T1 (tours, maisons à pans de bois, socle,
  silhouette nette) ; trébuchet plus net (mais référence fausse). → **adopté**.
- **Retexture v6-lite** : pas de gain → abandonné. Bombarde : la référence fausse a peint le fût en bois.
- **À ingérer sans nouveau coût** : `gen.glb` de castle_west, city_west, siege_tower (trébuchet : non,
  référence fausse). Planches de city_west et siege_tower pas encore regardées.

## État
- [x] banc fal (gen + retex) + planches
- [x] abonnement, MCP installé, crochet anti-clé
- [x] vague 0 (13/10) : castle_byz en 7.1 / 6 / 6-lite, solde 1 060 → 985 (barème API confirmé :
      30 / 30 / 15). Planche `~/dev/cent-ans-raw/meshy/sheets/castle_byz_models.jpg`, glb dans
      `~/dev/cent-ans-raw/meshy/castle_byz/meshy-<modèle>.glb`. Les trois écrasent T1 (créneaux, appareil
      de pierre, coupole à fenêtres, socle herbeux). 6-lite ≈ 6 ≈ 7.1 à distance de jeu ; 7.1 plus fin
      mais 2,7 M triangles (6 : 1,4 M ; 6-lite : 31 k, déjà au budget). **Proposition** : 6-lite pour le
      décor (≈ 65 objets/mois), 7.1 multi-vues pour les figurines. Go du joueur (13/10) : 6-lite décor, 7.1 figurines, enchaîner
      les vagues jusqu'à épuisement.
- [ ] vagues lancées (13/10) : `python3 tools/experiments/meshy_waves.py run|status`, file d'attente
      Meshy limitée à 10 tâches, registre `~/dev/cent-ans-raw/meshy/tasks/ledger.json` (reprise sans
      doublon), glb dans `~/dev/cent-ans-raw/meshy/<id>/meshy.glb`. bombard et trébuchet exclus
      (images à refaire). Ensuite : planche des modèles finis (demandée par le joueur), puis ingestion.
- [ ] ADR « Meshy pour les assets prioritaires » (remplace « Meshy : trop cher »
      de `docs/pipeline-assets-3d.md` § Payant et l'ADR 0140 sur ce point)
- [ ] vagues 1-7
- [ ] **mise en jeu (demandée par le joueur, à faire en dernier)** : décor par `cent-ans dn-ingest`
      (classe, taille et lacet repris de l'entrée actuelle de `data/art/dn_manifest.json`,
      `--model-3d meshy-6-lite`), figurines par `ga3_figures.py <recette> --glb <meshy.glb>` (vérifier
      que le rig suit les bras plus levés de l'a-pose Meshy, sinon régénérer sans pose_mode) ; puis
      import Godot + `smoke.gd`, commit du manifeste, `art models-update --no-publish` (publication
      de la release sur accord du joueur).

## Reprise (pause du 13/10, fin de session)
- **Fait** : mois 1 généré (52 modèles : vague 0 + 16 figurines 7.1 + 14 châteaux + 14 villes + 4 engins
  6-lite, 1 035 crédits, arrêt à 25 crédits avant siege_bombard_great_1450). Planches finales
  `~/dev/cent-ans-raw/meshy/sheets/final/wave{0..4}_*.jpg` montrées au joueur.
- **Mise en jeu faite** : `meshy_waves.py ingest --wave 0..4` → 52/52 ok (journal `tasks/ingest.log`),
  import Godot ok, `smoke.gd` OK, `test_ga3_figures_manifest.py` 4 passed. Manifeste + battle_ga3 commités.
- **Prochaine étape (accord de publication DONNÉ par le joueur le 13/10)** :
  `uv run --project tools cent-ans art models-update` (publie models-v<N>), puis commiter
  `data/art/dn_models_hosting.json`.
- Puis : galerie 3D (`object__image__meshy-<modèle>`), `docs/budget.md` (abonnement 20 €/mois, 1 060
  crédits ; coût ≈ 0,28 € par modèle 6-lite, 0,57 € en 7.1 ; même chose sur fal ≈ 43 $), ADR « Meshy pour
  les assets prioritaires ».
- Mois 2 (dès le 13/11) : tout en meshy-6-lite (passer la branche figurines de `submit()`), refaire les
  images bombard/trébuchet en Z-Image local, puis fin vague 4 et vagues 5-7. Le joueur envisage le plan
  Premium (3 000 crédits, 38 € sur le site ; ≈ 0,15-0,19 € par modèle 6-lite) : conseillé un mois seulement
  pour tout produire.

## Vague 9 — bourgs de campagne (13/10)
Les 21 familles `town` de `dn_campaign_models.json` (encore TRELLIS 1), meshy-6-lite, 315 crédits
(solde 1 785 avant). Références contrôlées sur planche (toutes conformes). Lancement :
`meshy_waves.py run --wave 9` (journal `~/dev/cent-ans-raw/meshy/run_wave9.log`), puis `sheets --wave 9`,
`ingest --wave 9`, `albedo_gain: 1.0` sur les entrées `town` comme les villes (ADR 0349).
**Fait (13/10)** : vague 9 (21 bourgs, `c0c248a2f`), vague 5 (14 abbayes et églises) et vague 10
(21 villages) générées, planches regardées (Meshy meilleur partout), ingérées `--faithful`, gain 1.
`meshy_waves.py ingest` passe désormais `--faithful` pour les vagues 2, 3, 5, 9, 10. Solde 930.
Tous les lieux de la carte de campagne sont en Meshy. Reste : banc fps sur machine calme
(`mx-campagne-meshy.md`), release models (accord du joueur).
