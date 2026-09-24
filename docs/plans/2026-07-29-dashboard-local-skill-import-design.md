# Dashboard compact et import des skills locales — Design

**Date :** 2026-07-29  
**Périmètre :** `skillreg-local`  
**Cible :** collaborateurs métier utilisant l’application desktop  
**Statut :** implémenté et validé localement

## Problème

Deux défauts empêchent l’expérience gérée de tenir sa promesse :

1. À la largeur minimale Tauri de 900 px, le contenu principal conserve sa largeur intrinsèque.
   Le dashboard déborde derrière le bord droit et masque l’interrupteur de mise à jour ainsi que
   l’action de vérification.
2. La migration actuelle ne reconnaît que les installations historiques inscrites dans
   `installed.json`. Les dossiers de skills locaux non suivis ne peuvent ni rejoindre le stockage
   canonique SkillReg, ni apparaître dans le dashboard.

Le poste de validation ne possède aucun manifeste v1, mais contient des dizaines de dossiers de
skills locaux. Le parcours v1 ne peut donc pas effectuer la bascule demandée.

## Décision produit

SkillReg ajoute un import local distinct de la publication et de l’installation depuis le registre.

- Une skill importée reste locale à l’ordinateur.
- Elle est copiée dans le stockage canonique SkillReg.
- Les emplacements agents compatibles deviennent des liens gérés vers cette copie.
- Elle apparaît dans l’accueil et dans « Mes skills ».
- Elle n’est jamais envoyée au registre et n’est jamais concernée par les mises à jour distantes.
- Un lien externe ou un conflit reste intact.
- L’option est disponible dans « Réglages » pour tout utilisateur connecté, y compris après
  l’onboarding.

Cette distinction doit rester invisible dans le parcours principal. L’utilisateur voit une skill
prête et les assistants dans lesquels elle est disponible, sans choisir agent, scope, chemin ou
version.

## Modèle de données

`ManagedSkill` reçoit une origine sérialisée :

```text
registry  skill installée depuis le catalogue et gouvernée par l’entreprise
local     skill importée depuis cet ordinateur, sans publication
```

La valeur par défaut est `registry` afin de lire les manifests v2 existants sans migration de
schéma. Les anciens binaires ignorent le nouveau champ. Une skill locale utilise :

- `skillId = null` ;
- `sourceOrg = "local"` ;
- `activeVersion = "local"` ;
- le hash de contenu comme empreinte locale ;
- le même manifeste et les mêmes bindings que les skills du registre.

Le cycle de mise à jour ignore les entrées `local` avant toute requête réseau. La réparation,
l’aperçu, la désinstallation et le changement d’organisation continuent d’utiliser les primitives
gérées existantes.

## Détection et aperçu

L’aperçu est strictement en lecture seule. Il inspecte les répertoires utilisateur préférés des
agents compatibles et groupe les entrées par nom :

- `importable` : dossier régulier avec un `SKILL.md` valide ;
- `duplicate_identical` : copies identiques, importées une seule fois ;
- `duplicate_divergent` : copies homonymes différentes, laissées intactes ;
- un lien local pointant vers une copie importable est repris avec elle ;
- `external_link_untouched` : lien vers une source extérieure, laissé intact ;
- `already_managed` : lien déjà détenu par SkillReg ;
- `invalid_untouched` : contenu incomplet ou illisible, laissé intact.

Le slug du dossier reste l’identité locale stable. Un ancien `SKILL.md` dont le champ `name`
contient un libellé humain différent reste importable sans réécriture. Les descriptions YAML
simples, littérales (`|`) et repliées (`>`) sont reconnues.

L’interface n’affiche aucun chemin. Elle montre seulement les comptes, les noms des skills
importables et les catégories qui resteront inchangées.

## Transaction d’import

Pour chaque groupe importable :

1. reprendre l’aperçu sous le verrou global des mutations gérées ;
2. copier la source dans un dossier `staging` sous le stockage canonique ;
3. refuser les liens ou entrées non régulières à l’intérieur du contenu ;
4. vérifier `SKILL.md` et comparer le hash source au hash de la copie ;
5. activer la copie canonique ;
6. renommer temporairement les dossiers et liens locaux reconnus ;
7. créer et vérifier les bindings SkillReg ;
8. écrire atomiquement le manifeste ;
9. supprimer les sauvegardes temporaires seulement après le commit.

Toute erreur avant le commit retire les nouveaux bindings, restaure les emplacements originaux et
retire la copie canonique. Une skill conflictuelle n’entre jamais dans la transaction.

## Interface publique

Une section « Rassembler mes skills locales » est toujours disponible dans les réglages :

- chargement de l’aperçu ;
- état vide « Aucune skill locale à importer » ;
- résumé avec nombre importable et nombre laissé intact ;
- action explicite « Rassembler N skills » ;
- progression non cliquable ;
- résultat avec nombre importé, ignoré et en erreur ;
- nouvelle prévisualisation et rafraîchissement du dashboard après succès.

L’ancien parcours de migration v1 reste inchangé pour les installations historiques suivies.

## Dashboard compact

Le correctif de largeur suit trois règles :

- le conteneur flex principal et sa zone scrollable reçoivent `min-width: 0` ;
- le dashboard et ses panneaux reçoivent également `min-width: 0` ;
- les actions, statuts et boutons passent en pile aux largeurs compactes au lieu de maintenir une
  largeur intrinsèque.

La sidebar conserve 224 px et ne rétrécit pas. À 900×600, toutes les actions restent visibles dans
la largeur disponible ; le défilement reste uniquement vertical.

## Critères d’acceptation

- Aucun débordement horizontal à 900×600.
- L’interrupteur et « Vérifier maintenant » sont visibles et utilisables à 900×600.
- L’option d’import est visible dans les réglages après onboarding.
- L’aperçu ne modifie aucun fichier.
- Les liens externes et conflits restent intacts.
- Une copie locale n’entraîne aucune requête de mise à jour au registre.
- Après import, une skill apparaît une seule fois dans le dashboard et reste disponible dans ses
  agents compatibles.
- Un échec simulé restaure les dossiers d’origine.
- Aucun contenu de skill n’est envoyé sur le réseau.

## Résultat de validation

La validation macOS arm64 du 2026-07-29 confirme :

- dashboard sans débordement horizontal à 900×600 ;
- interrupteur et action manuelle entièrement visibles à la largeur minimale ;
- option publique présente dans les Réglages après connexion ;
- 47 skills locales importées avec le statut `ready` ;
- 141 bindings créés pour Claude, Codex et Cursor ;
- un lien externe `decktype` détecté et laissé intact ;
- aucun conflit et aucune erreur pendant les trois vagues d’import ;
- nouvelle prévisualisation vide et compteur de 47 skills sur l’accueil après la bascule.

Un dossier vide sans `SKILL.md` et les fichiers isolés ne sont pas considérés comme des skills et
restent inchangés.
