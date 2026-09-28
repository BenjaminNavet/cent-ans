# 0098 — Titres féodaux au-dessus des factions (chantier FE)

Date : 2026-09-28. Statut : accepté. Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`.
Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.

## Contexte

Le départ est trop grand : le joueur royal commence avec tout son royaume et n'a guère de marge de
progression. Réduire les royaumes fausserait les frontières exactes de 1337. La vassalité actuelle
est un simple lien de faction à faction (`suzerain`, relations `vassal`/`overlord`) et deux champs
de province (`overlord`, `holder`) qui disent la même chose autrement, sans hiérarchie.

## Décision

1. **Titres *de jure*** (`data/titles/<id>.json`, schéma `title.schema.json`) : rang `kingdom`,
   `duchy` ou `county` (trois niveaux au plus, l'Empire est un royaume), titre suzerain de rang
   strictement supérieur, provinces du domaine propre, détenteur de 1337, objectifs historiques.
2. **Les factions détiennent des titres**, dont un titre principal (`primary_title`). Seules les
   détentions sont stockées dans l'état de campagne (`CampaignState::feudal`).
3. **Déductions jamais stockées** (`sim_campaign::feudal`) : suzerain d'une faction = détenteur du
   titre au-dessus de son titre principal ; suzerains d'une province = détenteurs de la chaîne de
   titres qui la contient (double allégeance de la Guyenne) ; arbre féodal.
4. **Maxime** : « le vassal de mon vassal n'est pas mon vassal ». On ne traite qu'avec son suzerain
   direct et ses vassaux directs (ost, tribut, protection, escalade).
5. **Paramètres** dans `data/rules/feudal.json` ; les anciennes constantes de `diplomacy.rs` en
   sont les valeurs par défaut.

## Conséquences

- `overlord` et `holder` sont retirés des provinces ; le script `tools/cent_ans_tools/feudal_migrate.py`
  a produit le registre initial (45 titres) à provinces constantes.
- La migration fait des princes d'Empire sans `suzerain` (Autriche, Bohême, Brabant, Gueldre,
  Hainaut, Waldstätten, Vérone) des vassaux *de jure* de l'Empire, puisque toutes leurs provinces
  en relevaient. Tant que F1 n'a pas branché la diplomatie sur les titres, le comportement reste
  celui du champ `suzerain` ; F1 aligne l'un sur l'autre et F8 juge l'équilibre.
- Format de sauvegarde 7 : les sauvegardes antérieures sont refusées avec un message explicite.
- Le registre définitif (≈ 130 factions, ≈ 250 provinces) viendra des lots F4a-F4e.
