# pain ai — Instruction.md (AI Builder Rules — READ BEFORE EVERY PROMPT)

> Ha file pratyek prompt chalvaychya ADHI vacha. Rules modle tar build break hoto.

## 1. Golden rules

1. **Ek prompt = ek slice.** Prompt madhe fakt tyacha Goal banva. Pudchya prompt cha kaam "savat" karu naka.
2. **Verify-then-next.** Pratyek prompt chya Acceptance gate shivay pudhe jau naka. Fail → fix → re-verify, magach pudhcha prompt.
3. **Chhote diffs.** Ekaveli 3 peksha jast files create/edit naka; motha refactor nahi. Surgical edits, simplest approach (Karpathy guideline).
4. **Gate bypass kadhich nahi.** Shell, file-write, GUI click/type, settings-change — sagle `gate::check()` madhunach. Per-feature ad hoc approval checks BAN ahet. Bypass yenar asel tar te bug ahe, feature nahi.
5. **Secrets = keychain only.** API keys/tokens plaintext JSON/Markdown/env-example madhe nahi. Placeholder strings (`sk-...`, `xoxb-...`) suddha commit naka. Grep-verify pratyek prompt nanatar.
6. **Design tokens law.** Hex fakt `src/theme/tokens.ts` madhe. Baki saglikade `{token.refs}` / theme variables. DESIGN.md cha Trinity check (cream + coral + dark navy) pratyek UI prompt madhe run kara.
7. **Hermes sidecar = reuse, rewrite nahi.** Agent loop/registry/memory/cron Python sidecar madhech rahtil. Rust madhe parat implement karu naka. Approval fakt delegate kara.
8. **YOLO/smart/off modes BAN (v1).** `approvals.mode` fakt `manual`. Config + code assert donhi theva.

## 2. Per-prompt ritual (exact order)

1. Ha file + tya prompt cha Goal + `Nothing-do.md` vacha.
2. Jean files la haat lavaycha tyanchi list kara (prompt madhe dili ahe).
3. Implement kara (minimal diff).
4. Acceptance checklist run kara (commands prompt madhe dile ahet) — output paste kara session madhe.
5. `context.md` madhe decision/note add kara (2–4 lines max).
6. Magach pudhcha prompt.

## 3. Fresh-session prompts (context compaction nanatar)

1. `PRD.md` §2 + §6, `Instruction.md` (ha file), `context.md` — he tigha vacha.
2. Mag chaloo prompt (P-number) vacha aani fakt tech implement kara.
3. Junya decisions parat open karu naka; badal have asel tar `context.md` Decision Log madhe entry + karan liha.

## 4. Forbidden moves (auto-reject)

- `gate::check` sodun direct `std::process::Command` / `enigo` click / settings write.
- `approvals.mode: smart/off`, `--yolo`, `bypassPermissions`, `acceptEdits` enable karne.
- Keys `config.json`/`.env`/markdown madhe takne.
- Hex codes theme baher vaprane; Copernicus/StyreneB licensed fonts embed karne (substitutes: Cormorant Garamond + Inter).
- macOS/marketplace/auto-classifier/sandbox/gateway features "savat" add karne (te `Nothing-do.md` madhe ahet).
- Multi-thousand-line single file; dead code sodne; `unwrap()` non-test code madhe (Rust) sodne.

## 5. Done-definition (pratyek prompt sathi)

- [ ] Acceptance commands green (output dakhavla)
- [ ] Secrets-grep clean
- [ ] Design check pass (UI prompts)
- [ ] `context.md` updated (2–4 lines)
- [ ] Kahi fail asel tar te FAIL mhanun declare kele (silent skip nahi)
