# JR — Croisés : la Croisade du Saint-Sépulcre (conception)

Date : 2026-10-02. Demande du joueur : « une nouvelle faction de croisés, pour Jérusalem (objectif),
faction à part entière et mécanique propre ». Autonomie totale (pas de jalon de validation).
Décision d'architecture : ADR 0165.

## 1. But

Une faction jouable de plus, `fac_crusaders`, qui ne se joue pas comme un royaume : elle n'a presque
pas de terre, vit des aumônes de la Chrétienté et de l'élan de son vœu, et n'a qu'un but, prendre
et tenir Jérusalem.

## 2. Ancrage historique (uchronie assumée)

- 1333 : Jean XXII prêche le *passagium generale* ; Philippe VI en est capitaine général.
- Mars 1336 : Benoît XII annule le passage ; la flotte rassemblée en Provence est dispersée.
- Pierre de la Palud, dominicain, patriarche latin de Jérusalem (1329-1342), ambassadeur auprès du
  sultan en 1329, prêche la croisade à son retour ; les patriarches titulaires administrent alors
  l'évêché de Limassol, à Chypre.
- Uchronie de jeu : en 1337, les croisés qui refusent d'être relevés de leur vœu ont suivi le
  patriarche à Limassol. Toutes les données de la faction portent `uncertain: true` et une note
  « uchronie de jeu ».

## 3. La faction (données)

| Champ | Valeur |
|---|---|
| id | `fac_crusaders` |
| nom | « Croisade du Saint-Sépulcre », court « Croisés », adjectif « croisé » |
| régime | `theocracy`, succession `elective` |
| chef | `chr_pierre_de_la_palud`, patriarche latin de Jérusalem |
| religion, culture | `rel_catholic`, `cul_french` |
| base | `set_limassol` (ville portuaire de `prov_cyprus`, dont la cité reste à `fac_cyprus`) |
| armes | d'argent à la croix potencée d'or cantonnée de quatre croisettes du même |
| relations | guerre : `fac_mamluks` ; alliance : `fac_cyprus` (hôte) ; paix : Hospitaliers, Papauté |
| départ | une armée à Limassol, une petite flotte de transport, trésor modeste |

Objectifs (bloc `victory` de la faction, chemin déjà existant pour France, Angleterre, Bourgogne) :
1. **Délivrer Jérusalem** : contrôler `prov_jerusalem`.
2. **Un port pour les pèlerins** : contrôler au moins une province côtière de Terre sainte
   (`prov_gaza`, `prov_safad`, `prov_tripoli`).
3. Tenir le tout `hold_turns` tours avant `end_year`.

## 4. Mécanique propre : la Ferveur

Une jauge de faction, 0 à 100, départ 60. Elle remplace l'assise territoriale : elle paie l'ost,
elle le recrute, elle le dissout.

### 4.1 Ce qui la fait bouger

| Cause | Effet |
|---|---|
| chaque tour (le vœu s'use) | −1 |
| bataille gagnée contre une autre foi | +6 |
| bataille perdue | −8 |
| colonie prise en Terre sainte | +10 |
| Jérusalem prise | +40, puis plancher à 50 tant qu'elle est tenue |
| passage prêché | +10 |
| guerre déclarée à une faction catholique | −30 |
| bataille livrée contre des catholiques | −10 |
| paix ou trêve avec le maître de Jérusalem sans tenir la ville | −2 par tour |

### 4.2 Ce qu'elle donne

- **Aumônes** : revenu par tour = `alms_base + alms_per_fervor × ferveur`, versé au trésor, visible
  comme une ligne de revenu. C'est l'essentiel du revenu de la faction.
- **Prêcher le passage** (action du joueur) : coûte de l'or, exige de tenir un port, délai de
  recharge en tours. Deux tours plus tard, un contingent de volontaires débarque dans ce port :
  `1 + ferveur / 25` unités tirées d'une table pondérée d'unités existantes (piétons, arbalétriers,
  sergents montés, chevaliers). Aucun nouveau type d'unité (pas d'actif à produire).
- **Élan de la Croix** : ferveur ≥ 70, bonus de moral aux armées de la faction ; ferveur < 30,
  malus.
- **Débandade** : ferveur < 20, chaque tour une part des hommes de chaque armée rentre chez elle.
  À 0, la proportion double.

### 4.3 Jérusalem prise

Évènement « Jérusalem délivrée » pour toutes les factions, capitale transférée à Jérusalem,
ferveur +40 et plancher 50, prestige. La victoire suit la règle commune (`hold_turns`).

### 4.4 IA

Quand la faction n'est pas jouée : l'IA prêche dès que l'action est disponible et que le trésor le
permet. Elle mène sa guerre contre les Mamelouks avec l'IA commune. Si l'IA commune ne sait pas
débarquer, un lot dédié lui donne un objectif outre-mer explicite (cible : port de Terre sainte le
plus proche, puis Jérusalem).

## 5. Architecture

- **Données** : `data/rules/crusade.json` (+ schéma) porte tout : faction concernée, province but,
  liste des provinces de Terre sainte, barème de ferveur, aumônes, passage, seuils. Aucun
  identifiant de faction ni constante de règle dans le Rust.
- **Modèle** : `data-model/src/entities/crusade.rs` (`CrusadeRules`), chargé dans `GameData`
  (optionnel : sans le fichier, la mécanique est inerte).
- **Règle** : `sim-campaign/src/crusade.rs` — état `CrusadeState { fervor, preach_cooldown,
  pending_passages, target_taken }` dans `CampaignState` (optionnel, `serde(default)` : les
  anciennes sauvegardes chargent sans croisade) ; `resolve_crusade` dans la fin de tour ; crochets
  appelés par bataille, capture et diplomatie ; commande `preach_passage` ; vue `crusade_view`.
- **Pont** : une vue (jauge, détail des causes du dernier tour, aumônes, état du passage) et une
  commande, sur le modèle des missions.
- **Godot** : un encart « Ferveur » dans le HUD de campagne, visible seulement pour la faction
  croisée : jauge, aumônes, bouton « Prêcher le passage », infobulle détaillée. Évènements en
  toasts et chronique.

Chaque unité se teste seule : le barème (tests Rust purs sur `CrusadeState`), les crochets (tests
d'intégration bataille/capture), la vue (test du pont), l'encart (test Godot headless).

## 6. Cas limites

- Limassol perdue : la faction survit tant qu'elle a une armée (règle commune) ; le passage exige
  un port, les aumônes continuent.
- Trésor insuffisant ou recharge en cours : commande refusée avec un motif en français.
- Contingent attendu et port perdu entre-temps : il débarque dans un autre port tenu, sinon il est
  perdu (évènement).
- Faction croisée détruite : l'état de croisade est figé, plus aucun effet.
- Joueur d'une autre faction : aucun encart ; seuls les évènements publics apparaissent.

## 7. Tests et recette

- Rust : barème, plancher, aumônes, refus de la commande, arrivée du contingent, débandade,
  ancienne sauvegarde sans croisade, partie de 40 tours sans panique.
- Python : schéma `crusade.json`, cohérence des identifiants cités.
- Godot : smoke, test de l'encart, une capture de contrôle.
- Sonde : 5 graines × 50 tours, la faction croisée IA ne fait pas banqueroute et ne disparaît pas
  avant le tour 20 ; la jouer en pilote 15 tours (débarquement possible à la main).

## 8. Hors périmètre

Nouveaux types d'unités et figurines, portrait généré payant, croisades d'autres factions (appel
papal général), ordres militaires rejoignant l'ost, Reconquista et croisades baltes.
