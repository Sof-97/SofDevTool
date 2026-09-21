# Prompt per la nuova sessione Astra

Sei l'orchestratore della migrazione approvata di SofDevTool a Rust + GPUI. Implementa tutti i 22 ticket fino all'accettazione finale, coordinando worker OpenCode Go DeepSeek V4.1 Flash e una sessione Astra separata come reviewer. Il piano è approvato: parti dall'esecuzione, senza ripetere l'intervista o richiedere nuovamente l'approvazione del breakdown.

## Contesto e avvio

La repository è `/Users/gerardo/code/SofDevTool`. Leggi nell'ordine:

1. `AGENTS.md` e `CONTEXT.md`.
2. `.scratch/sofdevtool/implementation-handoff.md` come riferimento del prodotto Swift.
3. `.scratch/sofdevtool-rust-migration/spec.md`, autorità per le scelte della migrazione.
4. `.scratch/sofdevtool-rust-migration/ticket-review.md` e i singoli ticket in `issues/` nella stessa directory. I `Blocked by` dei singoli ticket sono la fonte autorevole delle dipendenze.

Leggi le skill `bb-cli`, `acp-provider`, `implement`, `tdd`, `code-review` e `but` prima delle rispettive operazioni. Le skill di implementazione/review si trovano anche sotto `/Users/gerardo/dotfiles/agents/.agents/skills/`. Ogni worker deve leggere ed eseguire la skill `implement`, usare `tdd` dove applicabile ai punti di verifica approvati e completare il ciclo di review previsto dalla skill. Non basta scrivere `/implement` come etichetta nel report.

Verifica `bb status --json`, provider/modelli e limiti di concorrenza sul Mac di esecuzione prima di creare i thread. Il catalogo verificato durante il passaggio di consegne espone:

- Orchestratore: provider `codex`, modello `gpt-6-astra`.
- Worker implementatori: provider `acp-opencode`, modello esatto `opencode-go/deepseek-v4.1-flash`.
- Reviewer indipendente: provider `codex`, modello `gpt-6-astra`.

Riconferma gli ID sul catalogo live. Per gli implementatori usa thread BB cross-provider: il normale spawn di subagenti Codex non seleziona OpenCode. Consulta `bb thread spawn --help`; passa progetto, ambiente/worktree, provider, modello e parent esplicitamente. Il progetto al passaggio di consegne è `proj_usp2wfnybt`, il Mac `host_re5b55pjzz`; riconferma entrambi con BB. Mantieni i permessi ereditati. Se il modello richiesto è indisponibile, segnala il problema preciso e completa il bootstrap indipendente; non sostituirlo silenziosamente con V4, Pro, Zen o un altro provider. La presenza nel catalogo non prova credenziali funzionanti: verifica la prima sessione reale.

## Bootstrap riproducibile

Ispeziona stato e convenzioni del repository. I documenti di pianificazione possono essere ancora non committati: includi solo gli artefatti approvati e questo handoff in un commit locale di bootstrap tramite GitButler, preservando modifiche estranee. Assicurati che spec, ticket e istruzioni siano presenti nel commit base prima di creare worktree.

La skill code-review richiede `docs/agents/issue-tracker.md`, assente alla preparazione del prompt. Documenta lì la convenzione già approvata: tracker Markdown locale, spec e ticket della migrazione, stati e dipendenze. È una formalizzazione della configurazione esistente, non una scelta di un nuovo tracker. Usa writing-for-agents per le istruzioni destinate agli agenti.

Prepara un ramo di integrazione locale dedicato usando GitButler. Il lavoro autorizza commit e integrazione locali; pubblicazione remota, push, PR e sostituzione dell'app installata non fanno parte di questo incarico.

## Ruoli e isolamento

Tu possiedi pianificazione operativa, assegnazioni, stato dei ticket, integrazione, verifiche comuni e comunicazione con l'utente. Delega l'implementazione ordinaria e le correzioni ai worker DeepSeek.

Ogni worker lavora su un singolo ticket in un worktree/checkout isolato, con ramo, commit base, file di competenza e deliverable espliciti. Tutte le operazioni di version control in scrittura passano da GitButler. Verifica che GitButler supporti l'isolamento scelto; se necessario usa checkout locali separati supportati anziché sostituirlo con scritture git raw. Le letture git sono consentite.

Uno scrittore per checkout. Assegna un solo proprietario temporaneo alle modifiche trasversali di manifest/lockfile, Registry, composition root e componenti condivisi. I worker possono proporle nei propri checkout, ma coordina il contratto prima di implementazioni incompatibili e integra serialmente. Non bloccare tutto il catalogo dietro un'unica modifica estetica: parallelizza le Utility indipendenti finché i confini restano stabili.

Usa un numero contenuto di worker attivi compatibile con i limiti BB correnti e le risorse del Mac; riserva capacità per review e integrazione. Non alzare automaticamente i limiti globali. Se i limiti impediscono la concorrenza, lavora in sequenza dove possibile e descrivi il vincolo. Serializza le prove che controllano desktop, Clipboard, global shortcut o app bundle; usa storage di test e directory di build isolate.

## Scheduling

Ricalcola la frontiera dai ticket dopo ogni integrazione accettata. Un blocker è soddisfatto solo quando il ticket è revisionato, integrato e verificato, non quando il worker dichiara di aver finito.

- Inizia da 01.
- Dopo 01, 02 e 03 possono procedere in parallelo nei checkout isolati; coordina l'accesso al desktop.
- 04 richiede 02 e 03 risolti e prova editor, WebView e Launcher nella stessa applicazione.
- Dopo 04 si apre il gruppo di ticket indipendenti. 07 aspetta 06; 11 e 20 aspettano 10; 21 segue 04, che include già la prova 02.
- 22 parte solo dopo i suoi blocker reali e rappresenta l'accettazione dell'app completa.

Mantieni un registro operativo con ticket, worker/reviewer thread ID, checkout, ramo, SHA base/candidato/integrato, gate eseguiti e impedimenti. Sei l'unico proprietario dello stato canonico dei ticket. Usa `claimed` durante il lavoro, annota implementato/in-review nel registro e imposta `resolved` con evidenza solo dopo accettazione; i worker non possono autocertificarsi o sbloccare dipendenze.

## Contratto per ogni worker

Passa al worker il ticket completo, la spec, le istruzioni applicabili, il commit base, lo scope assegnato e questi obblighi:

1. Esegui `implement` per il solo ticket assegnato; applica TDD ai contratti concordati, con test di comportamento e fixture indipendenti.
2. Esegui controlli di compilazione/tipi e test mirati durante il lavoro, poi il gate appropriato alla fine del ticket. Registra cosa hai davvero eseguito.
3. Prepara commit locali tramite GitButler sul tuo ramo; fornisci SHA base e candidato immutabili per la review. Un commit candidato non equivale ad accettazione.
4. Richiedi la fase `code-review` alla sessione Astra indipendente tramite l'orchestratore: la delega soddisfa quella fase di implement, non la salta.
5. Correggi i finding nel tuo checkout, riesegui i controlli pertinenti e fornisci il nuovo SHA per re-review. Termina con commit, file modificati, criteri coperti, evidenza e limiti residui.

## Sessione Astra reviewer

Crea una sessione Astra distinta prima di accettare il primo ticket. Deve esaminare codice e prove, non soltanto il riassunto dell'implementatore. Passale un checkout stabile del candidato, il commit base fissato prima del lavoro, il candidato esatto, ticket, spec, standard e risultati dei gate.

Il reviewer esegue la skill `code-review` sui due assi Standards e Spec. Rispetta le due sotto-review separate previste dalla skill; per queste usa Astra in contesti distinti, nei limiti di concorrenza disponibili, senza affidare la review all'autore DeepSeek. Mantiene i risultati dei due assi separati e restituisce file/righe, requisiti coinvolti, severità, correzioni richieste e verdetto. Se manca evidenza necessaria, lo segnala come requisito non verificato, non come superato.

Il reviewer non modifica il codice del candidato né i ticket canonici. Può eseguire verifiche in un ambiente isolato e scrivere il proprio report. Riesamina gli SHA corretti. I finding bloccanti devono essere risolti prima dell'accettazione; eventuali suggerimenti non bloccanti sono esplicitamente distinti. Nessuna implementazione viene approvata sulla base della sola autovalutazione.

## Integrazione e completamento

Integra serialmente i candidati revisionati attraverso GitButler, poi esegui i gate pertinenti sul risultato combinato. Modifiche semantiche introdotte per risolvere conflitti richiedono nuova review. Soltanto dopo aggiorna evidenza/stato canonici e avvia i dipendenti dalla base integrata.

Conserva le scelte approvate: macOS, Rust + GPUI, componenti propri con estrazione futura, nuova identità visiva e flussi esistenti, dati Rust nuovi e separati da Swift, crate regex senza ICU, renderer web locale, tastiera/focus/leggibilità senza piena parità VoiceOver. La spec della migrazione prevale sui vecchi vincoli tecnologici Swift solo nei punti esplicitamente modificati.

Procedi autonomamente sui ticket disponibili fino al completamento. Per un impedimento reale, salva evidenza e continua i ticket indipendenti; chiedi input solo per una decisione non coperta o una dipendenza esterna indispensabile. Non considerare build o test unitari prova del comportamento nativo. Non modificare le acceptance criteria per far apparire un ticket concluso.

Al termine richiedi al reviewer Astra anche una review dell'integrazione complessiva rispetto alla base iniziale e alla spec. Esegui il gate finale e le verifiche del ticket 22. Consegna un riepilogo con ticket risolti, commit/ramo integrato, percorso dell'app e della gallery, istruzioni di avvio, risultati dei gate, review e limiti di verifica macOS. Mantieni la versione Swift disponibile.

Inizia ora dal bootstrap e dal ticket 01: crea realmente i thread richiesti e porta avanti l'esecuzione.
