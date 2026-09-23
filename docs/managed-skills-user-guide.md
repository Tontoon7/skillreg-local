# Guide collaborateur — utiliser les skills SkillReg

Ce guide décrit le parcours principal de l'application desktop. Il ne demande aucune connaissance
de la ligne de commande.

## Première connexion

1. Ouvrez SkillReg et choisissez **Se connecter avec le navigateur**.
2. Autorisez l'application dans la page qui s'ouvre.
3. Si vous appartenez à une seule entreprise, SkillReg la sélectionne automatiquement. Sinon,
   choisissez son nom dans la liste.
4. SkillReg détecte les assistants disponibles et vérifie les installations déjà présentes.
5. Lorsque **Vos assistants sont prêts** apparaît, ouvrez l'application.

Si SkillReg propose de simplifier d'anciennes installations, vous pouvez continuer ou reporter
l'opération. Les installations de projet, les dossiers externes et les skills modifiées ne sont
jamais déplacés automatiquement.

## Installer une skill

1. Ouvrez **Catalogue**.
2. Recherchez la capacité dont vous avez besoin.
3. Consultez sa description, ses résultats attendus et ses éventuels accès requis.
4. Cliquez sur **Installer**.

SkillReg choisit la version approuvée par votre entreprise et la rend disponible dans tous les
assistants compatibles détectés sur votre ordinateur. Vous n'avez pas à choisir une version, un
assistant ou un emplacement.

Si la skill a besoin d'un accès, par exemple une clé CRM, SkillReg vous demande uniquement les
informations encore manquantes. Elles restent stockées localement.

## Rassembler des skills déjà présentes

Si des skills se trouvent déjà sur votre ordinateur :

1. Ouvrez **Réglages**.
2. Consultez la section **Rassembler mes skills locales**.
3. Vérifiez le nombre de skills reconnues et les éventuels éléments laissés intacts.
4. Cliquez sur **Rassembler les skills**.

SkillReg conserve une seule copie locale de chaque skill reconnue et la rend visible dans
**Accueil** et **Mes skills**. Les copies identiques sont réunies automatiquement. Les conflits,
les liens vers un dossier externe, les dossiers incomplets et les fichiers isolés restent
inchangés.

Aucun contenu n'est envoyé et rien n'est publié. Une skill ainsi rassemblée reste locale et n'est
pas remplacée automatiquement par une version du catalogue.

## Comprendre l'accueil

L'accueil indique :

- si vos assistants sont prêts ;
- le nombre de skills disponibles ;
- les actions qui demandent votre attention ;
- si les mises à jour automatiques sont activées.

L'interrupteur **Maintenir mes skills à jour** contrôle toutes les skills gérées. Même lorsqu'il est
désactivé, **Vérifier maintenant** et **Tout mettre à jour** restent disponibles.

## Gérer « Mes skills »

Une skill apparaît une seule fois, même lorsqu'elle est disponible dans plusieurs assistants.

- **Configurer** complète un accès manquant.
- **Mettre à jour** applique une version approuvée.
- **Voir le problème** affiche l'état des connexions.
- **Réparer ce qui peut l'être** recrée uniquement les connexions dont SkillReg peut prouver la
  propriété.

Un dossier existant ou une connexion modifiée n'est jamais écrasé. SkillReg signale le conflit et
laisse les fichiers intacts.

## Désinstaller

1. Dans **Mes skills**, ouvrez le menu secondaire de la skill.
2. Choisissez **Désinstaller**.
3. Lisez les effets puis confirmez.

La skill n'est plus proposée aux assistants. Les accès enregistrés sont conservés pour une
réinstallation ultérieure. Leur suppression est une action séparée. Si une connexion locale a été
modifiée, la désinstallation s'arrête sans supprimer de fichier.

## Changer d'entreprise

Lorsque plusieurs entreprises sont disponibles, utilisez le sélecteur de l'accueil. SkillReg
retire d'abord les connexions de l'entreprise précédente, vérifie les skills déjà en cache, puis
active la nouvelle entreprise. Une erreur ou un conflit conserve l'état précédent.

Une seule entreprise expose ses skills aux assistants à la fois. Revenir à une entreprise déjà
utilisée réemploie les copies locales vérifiées.

## Fonctionnement local, en termes simples

SkillReg conserve une seule copie de chaque skill dans son espace local, puis crée des connexions
vers les assistants compatibles. Une mise à jour remplace cette copie de façon atomique, après
vérification de son identité et de son checksum. Les assistants n'ont donc pas chacun une copie
indépendante susceptible de diverger.

Si vous installez plus tard un nouvel assistant compatible, ouvrez simplement SkillReg :
l'accueil vérifie les connexions et rend automatiquement les skills déjà installées disponibles
dans ce nouvel assistant.

## Vie privée et usage

Le lot actuel ne collecte pas vos prompts, conversations, réponses ou fichiers. Il n'affiche pas
de compteur d'usage ni de suggestion de désinstallation automatique. Ces fonctions restent
différées jusqu'à validation d'un signal fiable et d'un contrat de confidentialité.

## Pour les utilisateurs avancés

Le CLI reste disponible pour publier des skills et installer des skills dans un projet. Sur une
machine gérée par le desktop, le CLI ne modifie pas les installations globales : il guide vers le
catalogue desktop afin d'éviter des copies concurrentes.
