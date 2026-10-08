# ADR 0211 — Charte semi-réaliste révisée (trois états, figurines générées, palette contrôlée)

Statut : accepté (08/10, arbitrages du directeur artistique, chantier DN).

## Contexte

L'audit DN (`docs/wip/dn/revision-bible.md`, `audit-campagne.md`, `audit-bataille.md`) a relevé des
contradictions dans la bible DA : maquettes stylisées d'ADR 0158 contre « une seule main », § 7 et § 8
périmés (ADR 0124, 0138), vue parchemin absente des deux registres, ADR 0136 interdisant les soldats
générés alors que la cuisson GA3 les anime déjà (CAM1 dans `battle_ga3/`), palette sans valeur de
clarté ni contrôle, budgets triangles limités aux figurines.

## Décision

- **D1** : les maquettes stylisées (ADR 0158) sont l'« état généralisé » du réalisme peint, réservé au
  lointain. Aux zooms proche et moyen de la campagne, les bâtiments sont des glb générés
  semi-réalistes, un jeu par famille d'architecture (cible de la nuit DN).
- **D2** : figurines générées autorisées en bataille **via la cuisson GA3** (auto-pondération sur le
  rig human/cavalry, CAM1 LOD0-2, livrée vert → alpha d'albédo, budgets LOD de la bible § 6) ; l'ADR 0136
  (« pas de soldats générés ») est levé pour ce chemin. Usage statique ailleurs (figurines d'armée de
  campagne, porte-bannières, morts et débris, imposteurs). Un glb TRELLIS brut non cuit n'est jamais
  animé. Rig direct sur le squelette partagé ou projection de texture sur le corps du kit : lot
  exploratoire séparé, pas une règle.
- **D3** : budgets triangles de la bible § 14.4 acceptés ; textures 2048² pour les bâtiments majeurs
  seulement, 1024² ailleurs (512² accessoires).
- **D4** : la vue parchemin (< 1200, ADR 0124) relève du registre enluminure ; deux registres
  seulement. Lot DA3 et « marqueurs de carte » (§ 8) abandonnés.
- **D5** : saturation S ≤ 0,40 hors livrées = règle, contrôlée automatiquement à l'étape image
  (moyenne et percentile 95 HSV sur le détouré, hors pixels vert/bleu purs de livrée), pas sur le rendu.

Application : bible révisée (§ 1 bis, 3.3, 6, 7, 8, 10, nouveau § 14), `style_prefix`/`style_suffix`
de `data/art/ga3_decor.json`, `docs/pipeline-assets-3d.md` § 2.

## Conséquences

- `target_albedo_mean` (bloc `cleanup`) n'est pas lu par les scripts : noté « à venir » dans la
  bible, absent du JSON. Le contrôle S automatique reste à outiller (étape IMAGE de `ga3_fal_decor.py`).
- Les prompts du catalogue décor changent : les objets déjà générés ne sont pas régénérés d'office.
- Pas de catalogue de figurines dans `data/art/` : le prompt figurines vit dans la bible § 14.8.
- Les maquettes d'ADR 0158 ne sont plus une dette « autre main » mais un niveau voulu.
