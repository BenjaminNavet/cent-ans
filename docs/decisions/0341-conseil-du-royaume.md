# 0341 — Conseil du royaume : charges, conseillers et salle 3D

## Contexte
Retour du joueur (RG, 11/10) : il veut « voir le conseil », avec les personnages conseillers, et y
trouver une aide à jouer (conseils pour diriger le royaume, missions, alertes). Le cœur n'a aucune
notion de conseil ; l'aide existe mais éparpillée (indice « Que faire maintenant », suivi des
missions, objectifs touche O, cloche d'alertes, conseiller vocal Jean le Bel). Les personnages ont
déjà des compétences `command` / `governance` / `court` (0-10) et des portraits.
Choix du joueur : une salle 3D (pas un tableau peint).

## Décision
1. **Charges** (cœur, `sim-campaign`) : cinq charges du conseil, définies dans
   `data/council/offices.json` (schéma `data/schemas/council_offices.schema.json`) :
   | Charge | Compétence | Effet (par point de compétence du titulaire) | Domaine de conseil |
   |---|---|---|---|
   | Chancelier | court | +1 point d'acceptation diplomatique | diplomatie, traités, croisade |
   | Connétable | command | +0,2 expérience des recrues, +0,5 % moral d'armée | armées, recrutement, menaces |
   | Grand trésorier | governance | +0,5 % des recettes fiscales | budget, chantiers, commerce |
   | Aumônier | court | −0,3 point de troubles par province | ordre public, religion, Église |
   | Chambellan | court | +0,1 prestige par saison | missions, objectifs, cour, succession |
   Les valeurs sont des données, pas des constantes. Charge vacante = aucun effet.
2. **Nomination** : ordre `AppointCouncillor { office, character }` (personnage vivant, adulte, de la
   faction ; une charge par personnage ; le souverain ne siège pas comme conseiller, il préside).
   `DismissCouncillor { office }`. Remplacement d'un titulaire : −1 loyauté au révoqué s'il y a une
   loyauté. À la mort d'un titulaire, la charge devient vacante (événement d'alerte). Au début de
   partie et pour l'IA, attribution automatique au meilleur candidat par compétence (déterministe).
3. **Conseils** : rendu seulement. Chaque alerte, indice, mission ou objectif déjà produit est rangé
   dans le domaine d'une charge (table de correspondance dans `data/council/offices.json`) ; le
   conseiller présente ceux de son domaine, triés par urgence. Aucune nouvelle règle côté `game/`.
4. **Salle 3D** (`game/`) : scène `council_chamber.tscn` dans un `SubViewport` ouvert depuis la
   barre (bouton « Conseil », touche K) : salle voûtée, table chargée de cartes, souverain en bout
   de table, cinq conseillers assis. Figures 3D génériques par charge (habit du métier : robe de
   chancelier, armure de connétable, etc.) produites par le pipeline local gratuit (ADR 0210) ;
   identité du titulaire par portrait, nom et blason au survol. Clic = fiche de la charge (titulaire,
   effet chiffré, conseils, bouton « Nommer… »). Charge vacante = siège vide.

## Conséquences
- Nouvelle couche de règles légère, sauvegardée dans l'état (compatibilité : charges absentes =
  attribution automatique au chargement).
- Les figures ne reflètent pas le visage du titulaire (portrait au survol) ; acceptable, et évite
  une génération 3D par personnage.
- L'aide à jouer éparpillée garde ses accès actuels ; le conseil en est la vue d'ensemble.
