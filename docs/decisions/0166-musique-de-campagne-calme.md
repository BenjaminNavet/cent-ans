# 0166 — Musique de campagne calme : luth, vihuela, luth-clavecin, violes

Date : 2026-10-02. Complète les ADR 0060 (musique d'époque) et 0154 (rotation).

## Contexte

Retour du joueur : la musique de campagne « n'est pas bonne ». Il veut du baroque plus détendu,
sans instrument moderne (luth, harpe…), pour que l'anachronisme ne s'entende pas. Constat : les
listes `primary` de campagne étaient surtout des chansons médiévales chantées (Studio der frühen
Musik) et des danses vives (estampie, saltarello, flûte et tambourin) ; en guerre — presque toute
la partie — la carte jouait en plus les pièces martiales de `war`, partagées avec la bataille.

## Décision

- **Fidélité d'instrument plutôt que de date.** La carte et la cour jouent des pièces calmes pour
  cordes pincées et instruments doux, du XVIe au début du XVIIIe siècle : luth (Le Roy, anonymes),
  vihuela (Mudarra), théorbe (Kapsberger), luth-clavecin (Bach, suites pour luth), clavicorde
  (Cabezón), clavecin (Gibbons), violes (Hume, Moulinié). L'anachronisme de répertoire est assumé :
  aucun timbre moderne, aucune voix, aucun synthétiseur (bible DA § 9 maintenue sur ce point).
- **Sources** : 17 enregistrements réels de Wikimedia Commons (CC0, CC BY, CC BY-SA ; `CC BY-SA
  2.0` ajoutée aux licences acceptées), ajoutés à `tools/cent_ans_tools/era_music.py`. Écartés :
  rendus au synthétiseur (Pracchia-78), albums d'Internet Archive à la licence douteuse.
- **Listes** (`data/audio/music.json`) : ces pièces deviennent le `primary` de `campaign`,
  `campaign_france|england|burgundy|iberia|italy` et `court` ; les anciennes pistes passent en
  `fallback` (jamais supprimées). `menu`, `campaign_orthodox` et `campaign_islamic` sont inchangés.
- **Carte en guerre séparée de la bataille** : `war` ne porte plus que trois pièces graves mais
  calmes (toujours complétées par la liste régionale, ADR 0154) ; les pièces martiales passent
  dans un nouveau contexte `battle`, lu par `BattleMusicDirector` (à défaut `war`).

## Conséquences

- +17 fichiers MP3 dans `game/assets/third_party/music/wikimedia/`.
- La harpe seule manque : rien de libre et d'époque sur Commons. À ajouter si une source sûre
  apparaît.
- Le répertoire n'est plus daté de la guerre de Cent Ans ; les pièces médiévales restent sur le
  disque et dans les listes `fallback`, on peut les remonter par simple édition de données.
