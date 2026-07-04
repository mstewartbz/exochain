# Phase 8 ICE Card Images

## Source Basis

- Source file checked on 2026-05-24:
  `/Users/bobstewart/Library/Mobile Documents/com~apple~CloudDocs/ICE-CARD-IMAGES.pdf`.
- macOS metadata reports one PDF page, Adobe PDF content type, and a file size
  of 2,150,084 bytes.
- Poppler command-line tools were unavailable in this environment, so the PDF
  was rendered with macOS Quick Look and inspected visually as PNG output.
- Rendered image used for inspection:
  `/Users/bobstewart/dev/livesafe/tmp/pdfs/ice-card-phase-8/ICE-CARD-IMAGES.pdf.png`.
- High-resolution crop used for close inspection:
  `/Users/bobstewart/dev/livesafe/tmp/pdfs/ice-card-phase-8/crops/card-visible-high.png`.
- Additional page images supplied by the user after the PDF inspection:
  `/Users/bobstewart/Library/Mobile Documents/com~apple~CloudDocs/IMG_9248.HEIC`
  through
  `/Users/bobstewart/Library/Mobile Documents/com~apple~CloudDocs/IMG_9255.HEIC`.
- macOS metadata reports each HEIC page image as 3024 by 4032 pixels with
  content type `public.heic`.
- HEIC inspection copies were converted to PNG under
  `/Users/bobstewart/dev/livesafe/tmp/images/ice-card-phase-8-pages/`.
- User-provided source context: this PDF is a remnant of the original concept
  from 2010; the old phone numbers and links no longer work; the card was
  generated from the user's account preferences; and the original experience
  encouraged the user to cut out the card and fold it along lines from
  instructions on the cover page and margins.
- Sensitive material handling: the visible card instance includes a real
  cardholder name, portrait, signature, printed contact/URL details, and QR
  code. This record treats them as evidence of field types and layout only. Do
  not reuse visible personal values as sample data. Do not transcribe or migrate
  the obsolete printed phone, link, or QR target values.

## Fact vs Inference

- Fact: the available PDF render is a single screenshot-like page that includes
  Adobe Acrobat mobile interface chrome, including a generative-summary banner,
  not a clean source export of the printable card.
- Fact: the HEIC page set shows the same worn physical artifact across multiple
  angles and unfolded states, making the packet structure clearer than the PDF
  screenshot.
- Fact: the visible artifact is a worn, cut, and folded wallet-card-sized
  physical card photographed against a plain surface.
- Fact: the card title reads `In Case of Emergency Card`.
- Fact: the card includes a round emergency/medical-style logo at the upper
  left. The small outer-ring words are only partially legible in the render.
- Fact: the card layout includes a large cardholder-name field, an effective
  date field, a membership/status level field, a portrait area, and scan
  instructions.
- Fact: the visible effective date on this card instance is `2020-09-15`.
- Fact: the visible level text is `HEROES`.
- Fact: the visible scan instruction says `Scan QR code with phone` and the
  next visible line ends with `camera for secure link.`
- Fact: the HEIC images show the QR code, a printed web/contact area, and text
  indicating official-use handling. Exact printed endpoint values are not
  recorded here because the user states they are obsolete.
- Fact: the HEIC images show a blue medical-symbol panel adjacent to the QR
  panel.
- Fact: the HEIC images show a dense `HIPAA Privacy Authorization & Medical
  Records Release Authorization` panel with numbered clauses for authorization,
  effective period, extent of authorization, use, termination, revocation
  rights, benefits, and disclosure.
- Fact: the HEIC images show a `Transfer on Death Deed` panel with a list of
  asset categories, a signature/date area, and a printed fold instruction.
- Fact: the HEIC images show a `Constitutional Rights Assertion Notice` panel
  with numbered statements, a signature/date area, and language about refusing
  to waive specified rights. This is recorded as historical artifact content,
  not as a validated current legal design.
- Fact: the card image shows creases, edge wear, smudging, and fold marks,
  supporting the user's statement that this was meant to be physically carried.
- Fact: the QR code itself is not visible in the inspected rendered PDF page,
  but it is visible in the later HEIC images.
- Fact: cover-page and margin instructions are not visible in the inspected PDF
  page, but the HEIC images show at least one fold instruction printed on the
  artifact.
- Fact: the user states that phone numbers and links in the old artifact no
  longer work and must not be treated as current product endpoints.
- Inference: the modern requirements should preserve the physical-card product
  pattern: account-driven generation, printable output, cut/fold guidance,
  wallet carry, QR-based secure retrieval, card versioning, and visible status.
- Inference: the QR should act as an activation or retrieval pointer, not as a
  raw store of sensitive emergency or medical data.
- Inference: the original packet combined at least four function types:
  identity/scan access, medical-record release, transfer/legacy declaration,
  and rights assertion. A modern product should model those as separate
  user-controlled modules instead of treating the whole packet as one generic
  card.
- Inference: the dated card instance likely reflects a later regenerated or
  photographed artifact even though the concept lineage is user-described as
  originating in 2010.

## Artifact Inventory

| Artifact | Type | Source location | Relevant concepts | Why it matters | Confidence | Recommended action |
| --- | --- | --- | --- | --- | --- | --- |
| `ICE-CARD-IMAGES.pdf` | PDF / visual artifact | `/Users/bobstewart/Library/Mobile Documents/com~apple~CloudDocs/ICE-CARD-IMAGES.pdf` | ICE card, printed wallet card, QR secure link, account-generated card | Direct visual remnant of the original physical-card concept | high | preserve pointer |
| Full rendered PNG | temporary visual render | `/Users/bobstewart/dev/livesafe/tmp/pdfs/ice-card-phase-8/ICE-CARD-IMAGES.pdf.png` | card face, Adobe UI wrapper, visible card fields | Used for careful visual inspection because PDF layout extraction tools were unavailable | high | regenerate from source when needed |
| Card close crop | temporary visual render | `/Users/bobstewart/dev/livesafe/tmp/pdfs/ice-card-phase-8/crops/card-visible-high.png` | title, logo, name field, date field, level field, portrait, scan instruction | Best local view of the visible card face | high | use as inspection aid |
| HEIC page set | image set | `/Users/bobstewart/Library/Mobile Documents/com~apple~CloudDocs/IMG_9248.HEIC` through `/Users/bobstewart/Library/Mobile Documents/com~apple~CloudDocs/IMG_9255.HEIC` | QR panel, medical release panel, transfer-on-death panel, rights assertion panel, signatures, fold instruction | Reveals the full multi-panel wallet packet that the PDF render only partially showed | high | preserve pointer |
| HEIC PNG inspection copies | temporary visual renders | `/Users/bobstewart/dev/livesafe/tmp/images/ice-card-phase-8-pages/IMG_9248.png` through `/Users/bobstewart/dev/livesafe/tmp/images/ice-card-phase-8-pages/IMG_9255.png` | visual review, full packet layout | Stable local copies for repeatable inspection without editing the originals | high | regenerate from source when needed |
| User Phase 8 statement | current-chat source | Codex thread, 2026-05-24 | 2010 origin, obsolete phone numbers and links, account-preference generation, cut/fold instructions | Supplies functional requirements not visible in the single rendered page | high | carry into requirements |

## Requirements Captured

- The physical emergency card is a first-class product artifact, not only a web
  page or runtime endpoint.
- The product must generate a printable card PDF from account preferences.
- Generated cards must support wallet carry, including a cut-out format and a
  foldable layout sized to sit behind a driver's license or similar ID.
- The printable output must include cover-page and margin instructions for
  cutting and folding.
- The card should include a visible card title, emergency/medical logo or brand
  mark, cardholder display name, optional portrait, effective date or version
  date, status or membership level, and QR scan instructions.
- The print packet should support multiple panels, including a front
  identity/QR panel, a medical release panel, a legacy or transfer directive
  panel, and an optional rights assertion panel.
- Each panel must be independently enabled, reviewed, versioned, and rendered so
  users do not accidentally publish legal or medical language they did not
  choose.
- Signature, printed-name, and date fields must be explicit for any panel that
  purports to authorize disclosure, transfer, or rights instructions.
- The QR flow must use a secure link or activation pointer. Raw sensitive data
  must not be embedded directly in the printed QR payload.
- Any phone number, URL, or activation link printed on a generated card must be
  validated at generation time and invalidated or rotated when endpoints change.
- Legacy phone numbers and links from the old artifact must be treated as
  obsolete and must not be imported as live endpoints.
- Printed web/contact fields must be configuration-driven and must support
  replacement without changing the user profile itself.
- Medical release language must be treated as jurisdiction-sensitive legal copy
  requiring explicit user acceptance, effective dates, expiry or termination
  rules, revocation instructions, and audit history.
- Legacy, property-transfer, or end-of-life directives must be modeled as a
  separate document class from emergency medical access.
- Rights assertion language must be modeled as an optional separate document
  class with clear scope, version, jurisdiction, and user confirmation.
- The scanner experience must be mobile-first and legible for a responder under
  time pressure.
- The scanned view must disclose only the authorized emergency subset for the
  current access context.
- The card generator must support regeneration when a user's emergency profile,
  contacts, consent settings, card status, QR target, or legal copy changes.
- The card must carry a clear legal/privacy area, either on the card back or in
  the printable packet, matching the active scan-time access rules.
- Card status must support active, expired, replaced, and revoked states.
- The printed artifact must support fold-order instructions, including a clear
  first-fold instruction where the layout requires it.
- Test fixtures must use synthetic names, portraits, contacts, dates, and URLs.

## Product Architecture Impact

- ICE card requirements must be tracked as a physical-product and print-output
  surface alongside the LiveSafe web app, the responder scan flow, and the
  EXOCHAIN-adjacent trust boundary.
- The card generator must be downstream of account preferences and emergency
  profile settings, not a static image.
- The card should remain functional when carried offline as a printed artifact,
  while the QR target resolves through current server-side access policy.
- The old artifact proves that user trust depends on durable physical handling:
  cut lines, fold instructions, print legibility, and carried-card wear must be
  part of acceptance testing.
- The HEIC page set broadens the physical product from a single emergency card
  face into a foldable personal legal and medical packet. The modern data model
  should separate those panels while allowing them to print as one packet.
- The QR and printed legal panels should share a common card version, effective
  date, and revocation state so a responder or authorized viewer can tell
  whether a carried card is current.

## Open Conflicts

- The user identifies the concept origin as 2010, while the visible card
  instance has an effective date of `2020-09-15`. Treat 2010 as lineage context
  and 2020-09-15 as the observed date on this particular card image.
- The PDF render did not show cover-page, margin, cut, and fold instructions;
  the HEIC images do show a printed fold instruction on the artifact. The
  original cover page and margin-instruction page remain unretrieved.
- The user says old phone numbers and links no longer work. No current endpoint
  should be derived from this PDF without independent verification.
- The HEIC images show the QR code, but QR payload structure, card token
  format, scan endpoint, and revocation behavior remain unverified because the
  printed code points to an obsolete legacy endpoint.
- The visible artifact includes real personal information, a portrait, a QR
  code, printed contact/link values, signatures, and dates. Use only synthetic
  fixtures for new product examples, screenshots, tests, and demos.
- The historical medical-release, transfer, and rights-assertion panels should
  not be copied directly into the modern product without current legal review,
  jurisdiction tagging, user acceptance, and version control.
