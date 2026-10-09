# 0214 — Les lieux de la carte affichent les glb générés, par sous-famille, à toute distance

Date : 2026-10-09 (`docs/wip/dn/maquettes.md`). Suite de l'ADR 0158 (maquettes), 0211 (charte) et 0212 (paquet).

## Contexte

Le joueur ne voyait « aucun asset créé » : les villes restaient des maquettes stylisées (toits orange,
murs blancs). Cause : `DnCampaignModels` donnait à `ModelLibrary.get_scene` le chemin
`dn/buildings/town_west`, or l'ingestion écrit `town_west_lod0/1/2.glb` ; le fichier n'existait jamais,
`_dn_index` rendait -1 et la maquette restait seule. En plus, la table ne couvrait que 17 couples type ×
famille alors que 70 modèles de lieux existaient (sous-familles nord, balte, hanséatique, ibérique,
mamelouke, etc.), et le glb ne remplaçait la maquette que sous 300 unités.

## Décision

1. **Nom de modèle** : `DnCampaignModels.model_name(entry)` ajoute `_lod<N>` (`lod` de l'entrée, sinon
   `defaults.lod` = 1) aux chemins sans suffixe.
2. **Sous-familles** : `subfamilies` dans `data/art/dn_campaign_models.json` (ordre = priorité ; règle
   de la famille de la province dont culture, région ou religion correspond ; `also` = repli). Clés de
   recherche : sous-famille, ses `also`, famille, `*`. 21 sous-familles, 78 modèles distincts branchés,
   toute province couverte pour chaque type où un modèle existe. La famille reste celle de
   `town_maquettes.json` (modèles stylisés de repli inchangés).
3. **Plus de maquette jouet là où un glb existe** : `defaults.near_distance` = 1300 (au-delà de la
   portée de tous les types) ; la maquette ne reste que pour le fondu de portée et pour les lieux sans
   glb ou sans paquet (ADR 0212).
4. **Albédo** : les albédos baked par TRELLIS sont sombres ; `defaults.albedo_gain` (2,4) est appliqué à
   une copie du matériau (rugosité 1) pour que les lieux se lisent sur le sol.
5. **Lisibilité à distance moyenne (120-220)**, tout dans `data/` : `town_maquettes.json` `sizes` ×1,25
   (facteur constant par type, ADR 0158), rapports d'accessoires et portées de moulins/fumées relevés ;
   `map_fauna.json` : portée 230 (fondu dès 150), taille tenue à l'écran `length_k` 0,03. Les arbres
   (DN-FORET) et les villes 1:1 de près ne sont pas touchés.

## Conséquences

- 2 138 lieux sur 2 147 affichent un glb (le reste : points d'intérêt sans type).
- Aucun nouveau glb, donc pas de dépense fal ni de nouveau paquet de modèles.
- Coût de rendu : tuiles de glb lod1 visibles jusqu'à la portée du type ; à mesurer sur machine calme.
