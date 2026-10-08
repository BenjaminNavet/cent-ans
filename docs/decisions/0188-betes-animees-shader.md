# 0188 — Bêtes animées en shader de sommets, masques par position

Date : 2026-10-08. Chantier AS, lot AS1. Détail : `docs/wip/as1.md`.

## Contexte
Les bêtes de la carte vivante (FK : bœufs, vaches, moutons, chevaux de trait, bêtes attelées) et
les chevaux au piquet des camps de bataille sont des maquettes rigides qui glissent ou restent
figées. Les figurines font quelques pixels : un clip squelettique serait invisible et coûteux
(un squelette par `MultiMesh`). Doctrine ADR 0187 : procédural en shader de sommets.

## Décision
- Le mouvement est calculé dans le vertex shader (`animal_motion.gdshaderinc` pour la carte,
  `camp_horse.gdshader` pour le camp) : pas en quatre temps, rebond, hochement, broutage et
  mâchonnement, queue, souffle, déhanchement, roues et cahot.
- Les masques (pattes, tête, queue, roues) viennent de la position locale et de mesures par
  modèle dans `data/fx/animal_motion.json`, pas d'un attribut de sommet écrit par Blender : les
  glb restent inchangés (pas de régénération, pas de risque sur les 17 modèles FK2), et un
  nouveau modèle demande seulement une entrée de données. Un test (`as1_test.gd`) vérifie que
  chaque masque touche de la géométrie.
- La cadence se déduit de la vitesse réelle de l'instance (`INSTANCE_CUSTOM.y`, m/s) divisée
  par la longueur de foulée du modèle ; la phase vient d'une graine par instance (position).
  Le cahot des charrettes a pour période la foulée de la bête attelée.
- Chevaux du camp : `MultiMesh` propre (`AnimalMotion.build_camp_horses`) à la place du lot
  `BuildingKit.Batch`, un matériau `ShaderMaterial` par surface (couleur et rugosité copiées).
- Drapeau A/B `--no-as1` (et `enabled` dans les données) : retour au rendu d'avant.

## Conséquences
- Coût : quelques dizaines d'opérations par sommet sur ~1000 sommets de bêtes ; aucun coût CPU.
- Les masques par position supposent la conformation des modèles actuels ; régénérer un modèle
  avec d'autres proportions oblige à mettre à jour ses mesures (le test le signale).
- Les chevaux du camp perdent les variantes `Ga3Kit` s'il en existait pour le genre `horse`
  (aucune au 08/10).
