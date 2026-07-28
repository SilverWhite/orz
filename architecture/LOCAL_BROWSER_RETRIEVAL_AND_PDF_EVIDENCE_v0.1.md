# Local Browser Retrieval and PDF Evidence Design v0.1

**Date**: 2026-07-28
**Status**: design draft
**Scope**: local-browser external retrieval, paper PDF acquisition, deterministic local evidence reading

## 1. Purpose

This document defines a local retrieval design for the CLI project. The main goal is to improve scientific accuracy by routing external retrieval through the user's local browser, while treating downloaded PDFs as stable, hash-addressed evidence objects.

The intended first version is not a general browser agent. It is a constrained evidence acquisition layer:

```text
User question
  -> CLI retrieval planner
  -> local browser opens/searches in background
  -> web sources are read as rendered text
  -> papers are downloaded as PDFs
  -> local PDF text layer is indexed
  -> model reads deterministic text slices
  -> answer cites source records and PDF pages
```

The browser is used for discovery, login continuity, navigation, and download. Scientific reading is performed against local files whenever the source is a paper, technical report, standard, or other PDF-first document.

## 2. Product Principles

1. **Local first**: external retrieval runs through the user's machine and browser session. Cloud relay is out of scope for this design.
2. **Evidence before answer**: important claims must be backed by opened sources, not search snippets.
3. **PDF for scientific sources**: papers and technical documents should be downloaded and read from local PDFs whenever possible.
4. **Quiet by default**: browser work should not steal focus, switch the user's active tab, or interrupt typing.
5. **Visible when needed**: the CLI must show retrieval progress and source status in ordinary, inspectable language.
6. **Deterministic reuse**: downloaded PDFs are stored by content hash, with page text and metadata persisted.
7. **Constrained browser ownership**: the AI may control tabs it created or tabs explicitly handed over by the user, not the whole browser.
8. **No credential exposure**: cookies, password fields, auth headers, local storage, and unrelated tabs are never returned to the model.

## 3. Non-Goals

The first version does not attempt to implement:

- OCR for scanned PDFs.
- Full visual browsing or screenshot-based reading.
- Cloudflare relay, remote sync, or remote browser control.
- Arbitrary Chrome DevTools Protocol exposure.
- Reading all existing browser tabs, history, bookmarks, cookies, or passwords.
- Automatic CAPTCHA solving.
- Automatic form submission, purchases, account changes, or file uploads.
- Universal support for every publisher-specific reader.
- Browser extension store packaging or multi-browser support.

If a PDF has no usable text layer, the system should report that it cannot read the document under the current text-only policy.

## 4. High-Level Architecture

```text
CLI Agent
  |
  | structured retrieval requests and progress events
  v
Browser Retrieval Broker
  |
  | native messaging or equivalent local bridge
  v
Browser Extension
  |-- Tab Manager
  |-- Page Reader
  |-- Navigation Tracker
  |-- Download Tracker
  |-- Permission Broker
  |
  v
Local Browser
  |-- user windows and tabs, not visible by default
  |-- AI research window or background AI-owned tabs

Local Evidence Store
  |-- PDFs by sha256
  |-- page text indexes
  |-- source records
  |-- retrieval task logs
```

The browser extension owns browser-specific operations. The CLI owns task planning, evidence policy, answer construction, and user-facing status.

The local evidence store owns deterministic paper objects. Once a paper PDF is downloaded and validated, subsequent model reads should use the local store rather than the browser page.

## 5. Components

### 5.1 CLI Retrieval Planner

Responsibilities:

- Convert a user question into retrieval tasks.
- Decide whether a source should be treated as a paper/PDF source or a normal webpage.
- Limit concurrency.
- Request specific browser operations.
- Receive page summaries, PDF records, and failure states.
- Maintain source visibility state before allowing claims to depend on a source.
- Emit progress events for the terminal UI.

The planner should avoid making scientific claims from search result snippets. Search results are discovery candidates, not evidence.

### 5.2 Browser Retrieval Broker

Responsibilities:

- Bridge CLI messages to the browser extension.
- Enforce task-level permissions before forwarding requests.
- Normalize browser events into stable CLI events.
- Apply response size limits and redaction.
- Track AI-owned tabs per task.
- Ensure cleanup runs at task end or failure.

The broker is the safety layer between model-directed actions and browser capabilities. Even if an upper layer requests too much, the broker should refuse operations outside the active task policy.

Broker messages must be authenticated and schema-checked before execution:

- accept commands only from the paired extension/native host relationship
- pin or verify the expected extension ID or local origin equivalent
- require `task_id` on every browser action
- attach an explicit capability set to each task
- reject messages that do not match the browser action schema
- never accept commands directly from webpage JavaScript
- log every accepted browser action with task ID, tab ID, action type, and policy result

### 5.3 Browser Extension

Responsibilities:

- Create background tabs or use a dedicated AI research window.
- Open search pages and source pages without stealing focus.
- Read rendered page text for normal web sources.
- Inspect links and metadata for candidate PDF downloads.
- Track navigation, redirects, and load states.
- Capture download completion events.
- Close AI-owned tabs after task completion.

Default extension permissions should be limited to the capabilities needed for the first version. High-risk permissions such as cookies, history, and debugger should not be part of the default path.

### 5.4 PDF Evidence Store

Responsibilities:

- Store original PDFs by SHA-256.
- Validate that a downloaded file is a real PDF.
- Extract text from the existing text layer.
- Persist page-level text and metadata.
- Record parser version and extraction status.
- Provide read-by-page, search, and citation lookup operations.

This store is the primary evidence base for scientific papers.

### 5.5 Source and Citation Manager

Responsibilities:

- Assign stable source IDs.
- Bind claims to source records.
- Keep original URL, final URL, redirect chain, access time, source type, and content hash.
- For PDFs, bind citations to document ID and page numbers.
- Preserve enough data for the user to reopen or inspect the evidence path.

## 6. User Experience

### 6.1 Quiet Background Retrieval

Default behavior:

```text
- do not focus the browser
- do not switch the user's current tab
- do not show new foreground windows
- create AI-owned tabs in a dedicated AI Research window by default
- show progress in CLI
- close AI-owned tabs after retrieval
```

The default implementation should prefer a dedicated AI Research window over mixing task tabs into the user's active browser window. The window may be lazily created and kept unfocused or minimized. This makes cleanup simpler and reduces the chance that browser retrieval disrupts ordinary browsing.

Example CLI status:

```text
Retrieving: non-Markovian LIF memory kernel
  Search results checked: 8
  Sources opened: 4
  Paper PDFs found: 2
  Downloading: source_03
  Indexing PDF: pages 1-18
  Verifying: DOI/title match
```

### 6.2 Escalation to User Attention

The browser should come to the foreground only after user action or explicit need:

```text
Login required: Wiley Online Library
  [Open page] [Skip source] [Find another version]
```

Other attention states:

- CAPTCHA required.
- Download blocked by browser.
- Publisher access denied.
- Ambiguous paper version.
- Form submission would be required.
- The page appears to be an account, payment, email, or admin page.

### 6.3 Source Inspection After Cleanup

Closing task tabs must not erase the evidence path.

For each cited source, the CLI should preserve:

- title
- source type
- original URL
- final URL
- access time
- content hash or PDF SHA-256
- DOI when available
- PDF page citation when available
- local file ID when available

When the user opens a citation:

```text
if PDF exists locally:
  open local PDF, ideally at cited page
else:
  reopen final URL
```

## 7. Retrieval Modes

### 7.1 Normal Web Mode

Used for:

- documentation pages
- news and announcements
- ordinary webpages
- project pages
- API docs
- blogs

Flow:

```text
search/open page
  -> wait for rendered page
  -> inspect metadata and headings
  -> read relevant text blocks
  -> record source URL and content hash
  -> close tab unless user asks to keep it
```

Evidence level:

```text
B-level evidence: opened/read webpage with URL, access time, content hash, and usage trace.
```

Normal webpages should not be archived as full-text snapshots by default. The system records that the source was opened and used, plus stable fingerprints of the page and the used regions. This reduces hallucinated source claims without turning ordinary web browsing into a large local web archive.

### 7.2 Paper/PDF Mode

Used for:

- academic papers
- preprints
- technical reports
- standards
- white papers where PDF is canonical

Flow:

```text
search/open page
  -> identify DOI and PDF links
  -> download PDF through current browser login state
  -> validate file
  -> store by SHA-256
  -> extract text layer
  -> build page index
  -> model reads selected pages/sections
  -> cite document ID and page
```

Evidence level:

```text
A-level evidence: local PDF with fixed SHA-256 and page-addressable text.
```

The system should classify the downloaded document version as early as possible. A DOI identifies a work, not necessarily one exact file. The downloaded PDF may be the publisher version, an accepted manuscript, a preprint, supplementary material, or an unrelated access/error artifact. The evidence store records both a `version_guess` and `version_confidence`, and citations bind to the exact `document_id`.

### 7.3 Search Result Mode

Used only for discovery.

Search snippets may help rank candidates, but should not support scientific conclusions.

Evidence level:

```text
C-level evidence: discovery metadata only.
```

## 8. Browser Ownership and Permissions

### 8.1 Tab Ownership

The extension maintains an owned-tab registry:

```json
{
  "task_id": "retrieval_20260728_001",
  "tabs": [
    {
      "tab_id": 117,
      "owner": "agent",
      "created_at": "2026-07-28T14:30:00+08:00",
      "readable": true,
      "controllable": true
    }
  ]
}
```

Allowed tab classes:

1. AI-created tabs.
2. User-handed-over active tab.

Disallowed:

- Enumerating all user tabs.
- Reading titles of unrelated tabs.
- Reading unrelated browser windows.
- Accessing browsing history.

### 8.2 Default Allowed Actions

For AI-owned tabs:

- open URL
- go back/forward within task tab
- read title, URL, rendered text, headings, links, metadata
- scroll
- click ordinary links
- find text
- download candidate PDF
- close tab

### 8.3 Restricted Actions

Default block or user confirmation:

- submit form
- upload file
- send message
- change account settings
- authorize third-party app
- purchase or pay
- delete content
- access local files
- access localhost, private network, or cloud metadata addresses
- run arbitrary JavaScript with side effects

### 8.4 Never Returned to Model

- cookies
- saved passwords
- authorization headers
- hidden form fields
- password field values
- localStorage and sessionStorage
- full request headers
- unrelated tab titles or contents
- browser history

## 9. Browser Reading Strategy

### 9.1 Default Page Read

The first version should prefer extension-owned read functions instead of arbitrary JavaScript.

Suggested functions:

```text
get_page_outline(tab_id)
get_visible_text(tab_id, max_chars)
get_element_text(tab_id, selector)
get_links(tab_id)
get_metadata(tab_id)
get_json_ld(tab_id)
get_citation_metadata(tab_id)
find_text(tab_id, query)
get_surrounding_text(tab_id, query, before, after)
```

The model can ask for specific read functions, but the broker generates and runs the actual page script.

### 9.2 Exploratory Evaluation

An optional later tool can support constrained exploratory reads, for example:

```text
browser.evaluate_readonly(tab_id, expression)
```

This should be treated as a higher-risk mode. It is useful for unfamiliar page structures, but JavaScript cannot be made truly read-only by keyword filtering alone.

Constraints:

- only AI-owned tabs
- timeout
- max result size
- no cookie/local storage return
- no non-GET fetch
- no navigation
- no window creation
- execution logged
- URL checked before and after

This mode is not required for the MVP.

## 10. PDF Evidence Store Design

### 10.1 Storage Layout

Suggested layout:

```text
evidence/
  papers/
    sha256_prefix/
      sha256_full/
        original.pdf
        metadata.json
        pages.jsonl
        outline.json
        extraction.json
        source_record.json
```

Example:

```text
evidence/papers/82/82d1...a94f/original.pdf
```

### 10.2 Metadata

```json
{
  "document_id": "sha256:82d1...",
  "work_id": "doi:10.1234/example",
  "title": "Paper title",
  "authors": ["A. Example", "B. Example"],
  "year": 2025,
  "version": "publisher",
  "source_url": "https://doi.org/10.1234/example",
  "final_url": "https://publisher.example/article",
  "downloaded_at": "2026-07-28T14:35:00+08:00",
  "bytes": 1234567,
  "sha256": "82d1...",
  "pages": 18,
  "has_text_layer": true
}
```

### 10.3 Page Text Index

`pages.jsonl` should contain one JSON object per page:

```json
{"page":1,"text":"...","char_count":3210}
{"page":2,"text":"...","char_count":4188}
```

The model should read pages or search hits, not the entire PDF by default.

### 10.4 Extraction Record

```json
{
  "parser": "pdf-text-parser",
  "parser_version": "0.1",
  "extracted_at": "2026-07-28T14:36:00+08:00",
  "valid_pdf": true,
  "has_text_layer": true,
  "page_count": 18,
  "warnings": []
}
```

If no text layer exists:

```json
{
  "valid_pdf": true,
  "has_text_layer": false,
  "status": "unreadable_without_ocr"
}
```

## 11. Source Records

Source records should be separate from answer text. They are reusable audit objects.

```json
{
  "source_id": "src_004",
  "task_id": "retrieval_20260728_001",
  "source_type": "paper_pdf",
  "evidence_level": "A",
  "query": "non-Markovian LIF memory kernel",
  "title": "Paper title",
  "original_url": "https://doi.org/10.1234/example",
  "final_url": "https://publisher.example/article",
  "redirect_chain": [
    "https://doi.org/10.1234/example",
    "https://publisher.example/article"
  ],
  "accessed_at": "2026-07-28T14:35:00+08:00",
  "doi": "10.1234/example",
  "document_id": "sha256:82d1...",
  "work_id": "doi:10.1234/example",
  "version_guess": "publisher",
  "version_confidence": 0.86,
  "citation_locations": [
    {
      "page": 7,
      "section": "Methods",
      "evidence_role": "method",
      "quote_hash": "sha256:..."
    }
  ]
}
```

For normal webpages:

```json
{
  "source_id": "src_008",
  "task_id": "retrieval_20260728_002",
  "source_type": "web_page",
  "evidence_level": "B",
  "title": "Documentation page",
  "original_url": "https://example.com/docs",
  "final_url": "https://example.com/docs",
  "accessed_at": "2026-07-28T15:00:00+08:00",
  "content_hash": "sha256:...",
  "usage_trace": {
    "used_block_hashes": ["sha256:...", "sha256:..."],
    "selectors_or_headings": ["main article", "h2: Installation"],
    "char_ranges": [[1200, 1840], [3100, 3520]]
  }
}
```

The normal webpage record intentionally omits full text by default. It stores where the answer used the page and hashes of the used regions. If the page is later reopened and the hash no longer matches, the CLI should report that the source changed and the old web content cannot be fully reconstructed.

## 12. Claim-Evidence Binding

Answer text should not only cite source IDs. Important claims should be bound to evidence records in a structured form.

```json
{
  "claim_id": "claim_003",
  "claim_text": "The method estimates the memory kernel from trial-level voltage histories.",
  "claim_type": "scientific_method",
  "evidence": [
    {
      "source_id": "src_004",
      "document_id": "sha256:82d1...",
      "page": 7,
      "section": "Methods",
      "text_span_hash": "sha256:...",
      "evidence_role": "method"
    }
  ],
  "support_level": "direct"
}
```

Suggested evidence roles:

- `definition`
- `method`
- `result`
- `limitation`
- `background`
- `implementation_detail`
- `current_fact`

Scientific claims should not infer methods, results, or limitations from abstracts alone unless the answer explicitly labels the support as abstract-only or partial.

## 13. Source Visibility Gate Mapping

This design should integrate with the existing source visibility rule rather than define a parallel evidence system.

Suggested mapping:

```text
C-level search result:
  visibility = metadata_only
  allowed use = discovery only

B-level opened webpage:
  visibility = opened_read
  allowed use = ordinary web/documentation claims
  stored evidence = URL, access time, content hash, usage trace

A-level local PDF:
  visibility = full_text_available
  allowed use = scientific claims when page-level evidence is attached
  stored evidence = original PDF, SHA-256, page text index, source record
```

For scientific claims, the gate should prefer:

```text
downloaded/indexed PDF + document_id + page citation
```

and block or defer claims that depend only on:

```text
search snippet
abstract-only page
metadata-only DOI page
unvalidated download
PDF with no text layer
```

## 14. Tool Interface Draft

### 14.1 Browser Tools

```text
browser.search(query, options)
browser.open(url, background=true, task_id)
browser.inspect(tab_id)
browser.read_page(tab_id, options)
browser.find_text(tab_id, query)
browser.list_links(tab_id)
browser.find_pdf(tab_id)
browser.download_pdf(tab_id, link_id?)
browser.close(tab_id)
browser.close_task_tabs(task_id)
```

### 14.2 PDF Tools

```text
pdf.import_download(download_id, source_record)
pdf.index(document_id)
pdf.read_pages(document_id, start_page, end_page)
pdf.find(document_id, query)
pdf.get_metadata(document_id)
pdf.open_local(document_id, page?)
```

### 14.3 Source Tools

```text
source.record_web_page(record)
source.record_pdf(record)
source.attach_citation(source_id, location)
source.list_for_task(task_id)
source.open(source_id)
```

## 15. Task State Machine

```text
QUEUED
  -> SEARCHING
  -> OPENING_SOURCES
  -> READING_WEB
  -> FINDING_PDF
  -> DOWNLOADING_PDF
  -> VALIDATING_PDF
  -> INDEXING_PDF
  -> VERIFYING_SOURCE
  -> ANSWER_READY
  -> CLEANING_TABS
  -> DONE
```

Exceptional states:

```text
LOGIN_REQUIRED
CAPTCHA_REQUIRED
USER_CONFIRMATION_REQUIRED
SOURCE_UNAVAILABLE
PDF_NOT_FOUND
INVALID_PDF
NO_TEXT_LAYER
PAGE_BLOCKED
POLICY_BLOCKED
TIMEOUT
PARTIAL_EVIDENCE
```

All exceptional states should be explicit in the CLI. They should not silently degrade into unsupported claims.

## 16. Validation Rules

### 16.1 Download Validation

A downloaded PDF candidate must pass:

- file starts with a PDF signature
- parser can determine page count
- file size is not suspiciously tiny unless explicitly allowed
- page text extraction succeeds or reports no text layer
- title/DOI metadata roughly matches the target when known

If a publisher returns an HTML login page saved as `.pdf`, the result must be rejected.

### 16.2 Claim Gating

Suggested evidence policy:

```text
Scientific claim:
  prefer A-level PDF evidence
  allow B-level only when PDF is unavailable and webpage is the canonical source

Current fact or documentation claim:
  require B-level opened webpage or stronger

Search discovery:
  C-level only; cannot support important claims
```

### 16.3 Version Handling

Do not use DOI as the only identity.

```text
work_id: represents the research work, often DOI-based
document_id: represents the exact downloaded file, SHA-256-based
```

One work may have multiple documents:

- publisher version
- accepted manuscript
- preprint
- supplement
- correction

Citations must bind to `document_id`, not only `work_id`.

## 17. Access and Use Boundaries

This feature should use the user's existing legitimate browser access. It should not be designed as a paywall bypass or bulk scraping system.

Default policy:

- do not evade paywalls
- do not automate CAPTCHA solving
- do not share downloaded PDFs outside the local evidence store by default
- do not batch-download large collections without explicit user intent
- respect publisher rate limits and ordinary user-scale access patterns
- report access failure rather than silently substituting unsupported evidence
- keep local PDFs for the user's evidence workflow, not redistribution

The browser can use existing login state to download a paper the user is allowed to access, but the system should not extract or transmit the credentials that make that access possible.

## 18. Concurrency and Load Limits

Conservative defaults:

```yaml
browser:
  max_open_tabs: 4
  max_loading_tabs: 2
  close_tabs_after_task: true
  task_timeout_seconds: 60

pdf:
  max_parallel_parsers: 1
  max_pdf_bytes_default: 100MB
  no_ocr: true

text:
  max_page_read_chars: 100000
  max_search_results_opened_initially: 4
```

The system should use a pipeline:

```text
open a few candidates
  -> classify
  -> close low-value pages
  -> open more only if needed
```

Avoid opening many search results at once. This reduces browser load and keeps model context focused.

## 19. Security and Abuse Boundaries

### 19.1 URL Blocks

Default block:

- `file://`
- browser internal pages
- localhost
- private IP ranges
- cloud metadata addresses
- known email, payment, password manager, and account settings surfaces unless explicitly handed over by the user

Each navigation must be checked after redirects, not only before the first request.

### 19.2 Prompt Injection Boundary

Webpage text is evidence, not instruction. A page cannot grant new browser permissions, request unrelated tab reads, or cause local file access.

The broker should treat model-requested browser actions as policy-checked operations, not as direct execution of webpage instructions.

### 19.3 JavaScript Boundary

The MVP should use predefined read functions. Arbitrary JavaScript should be deferred.

If later added, arbitrary evaluation must be:

- logged
- size-limited
- timeout-limited
- restricted to AI-owned tabs
- unable to return credentials
- unable to submit forms or perform non-GET network writes by default

## 20. Test Matrix

The MVP should include fixture or integration tests for these cases:

| Case | Expected Result |
| --- | --- |
| Normal PDF with text layer | Valid A-level evidence; page text index created |
| `.pdf` filename containing HTML login page | Rejected as invalid PDF |
| PDF with no text layer | Explicit `NO_TEXT_LAYER`; OCR not attempted |
| DOI redirect chain | Original URL, final URL, and redirect chain recorded |
| HTML full text but no PDF | B-level or partial evidence; no full-paper claim unless allowed |
| Search result snippet only | C-level discovery; cannot support important claim |
| Page prompt injection | Treated as webpage text; no permission escalation |
| Redirect to localhost/private IP | Blocked after redirect check |
| Oversized PDF | Requires explicit policy decision or user confirmation |
| Same DOI, multiple PDFs | Separate `document_id` records with version guesses |
| Login required | User-visible `LOGIN_REQUIRED` state |
| CAPTCHA required | User-visible `CAPTCHA_REQUIRED` state |

## 21. Failure Handling

Failure should produce useful status, not quiet hallucination.

Examples:

```text
PDF not found:
  The source was opened, but no PDF candidate was found.

Invalid PDF:
  The downloaded file is not a valid PDF. It may be a login or error page.

No text layer:
  The PDF appears to be scanned or otherwise has no extractable text layer. OCR is disabled.

Login required:
  The page requires user login. User action is required before retrying.

Partial evidence:
  Only abstract or metadata was available. Do not use as full-paper evidence.
```

## 22. MVP Implementation Sequence

### Phase 1: Evidence Store

- Define PDF storage layout.
- Implement PDF validation.
- Implement text-layer extraction.
- Implement page-level read and search.
- Implement source record JSON.
- Implement version guess fields for downloaded PDFs.

This can be built and tested without browser automation.

### Phase 2: Browser Background Retrieval

- Add browser extension skeleton.
- Add local broker connection.
- Create background AI-owned tabs.
- Read rendered webpage text and metadata.
- List links and identify PDF candidates.
- Close task tabs.

### Phase 3: PDF Download Flow

- Download candidate PDF through browser.
- Pass completed download to evidence store.
- Validate and index.
- Return document ID to CLI.

### Phase 4: CLI Progress and Claim Gate Integration

- Display retrieval state.
- Surface exceptional states.
- Attach source records to answer packets.
- Prevent scientific claims from depending only on search snippets.
- Integrate A/B/C evidence levels with the source visibility gate.
- Add structured claim-evidence bindings for scientific claims.

### Phase 5: Publisher and Edge-Case Hardening

- Add small adapters only for frequently used publishers.
- Improve DOI/title matching.
- Add better failure classification.
- Add user handoff for login and CAPTCHA states.

### Phase 6: Test Matrix and Regression Fixtures

- Add fixture tests for invalid PDFs, no-text PDFs, redirects, prompt injection, and multi-version papers.
- Add integration tests for AI-owned tab cleanup.
- Add source visibility gate tests for C-level snippet blocking.
- Add normal webpage usage-trace tests without storing full-text snapshots.

## 23. Acceptance Criteria for First Usable Version

The first usable version should demonstrate:

1. A user query triggers a browser-backed search without stealing focus.
2. The system opens at most a small number of AI-owned tabs.
3. A normal webpage can be read as rendered text and recorded as B-level evidence without storing a full-text snapshot by default.
4. A paper PDF can be found, downloaded, validated, hashed, and stored locally.
5. The PDF text layer can be read by page.
6. The final answer can cite a source record and PDF page, with structured claim-evidence binding for scientific claims.
7. AI-owned tabs are closed at the end of the task.
8. Search snippets alone cannot pass the source visibility rule for scientific claims.
9. Login/CAPTCHA/no-text-layer cases produce explicit user-visible states.
10. The broker rejects browser actions without valid task IDs and capability authorization.
11. Multi-version PDFs under the same DOI are stored as separate document IDs.

## 24. Open Questions

1. Which browser is the first target: Chrome, Edge, or both?
2. Should the AI research window be lazily created, always kept alive, or user-configurable?
3. What exact hash granularity should normal webpage usage traces use: selected text block, rendered main text, or selector-local text?
4. What local path should be the default evidence library root?
5. How should citation opening work on Windows for page-specific PDF jumps?
6. Which PDF parser should be the first implementation target?
7. Should exploratory JavaScript evaluation be postponed until after the PDF flow is stable?
8. How strict should `version_confidence` be before a document can be treated as the publisher version?

## 25. Current Recommendation

Build the first version around this narrow loop:

```text
browser.search
  -> browser.open
  -> browser.read_page
  -> browser.find_pdf
  -> browser.download_pdf
  -> pdf.index
  -> pdf.read_pages / pdf.find
  -> source.record
  -> browser.close_task_tabs
```

This loop is small enough to implement without turning the browser into an unrestricted agent, but strong enough to materially improve scientific retrieval. It also matches the project direction: local, evidence-constrained, auditable, and careful about claim boundaries.
