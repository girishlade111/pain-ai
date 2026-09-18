# pain ai — Nothing-do.md (Scope Guard — v1 madhe HE KARU NAKA)

> AI builder la: khalil kahi "savat changla vatel" mhanun add kele tar te scope violation ahe. Pratyek prompt he check karel.

## 1. Platforms & distribution (v2+)

1. macOS app build / signing / notarization.
2. Mobile apps, browser extensions, web-hosted version.
3. Auto-update channels beyond stable signed artifacts; beta/nightly tracks.

## 2. Autonomy & safety downgrades (FORBIDDEN v1)

4. `approvals.mode: smart / off`, `--yolo`, `HERMES_YOLO_MODE=1`, kuthlahi auto-approve classifier.
5. `bypassPermissions`, `acceptEdits`, `auto`, `dontAsk` modes enable karne.
6. System/settings category sathi "Allow Always" dakhavne kinva allow karne.
7. Hardline blocklist (disk format, registry wipe, `rm -rf / ~`, firmware/BIOS, mass-cloud-delete) override karnyacha marg.
8. Whole-process sandboxing cha khota दावा — v1 madhe te NAHI he `PRD.md` madhe document ahe; gate + manual mode हाच v1 cha posture.

## 3. Product surface (v2+)

9. Sprite/MCP marketplace, public skill publishing, ratings/reviews.
10. Messaging gateways (Telegram/Discord/Slack/WhatsApp/Signal) — Hermes madhe ahet, pain ai v1 tyanna touch karat nahi.
11. Cloud sync, accounts, telemetry/analytics, crash-reporting with PII.
12. Multi-user / team / org features, roles, managed policies, remote control.
13. Second chibi character, paid voices, theme store.

## 4. Engineering temptations (reject)

14. Hermes agent loop/registry/memory/cron Rust madhe re-implement karne.
15. Docker/SSH/cloud terminal backends enable karne.
16. Licensed fonts (Copernicus/StyreneB) embed/download karne — substitutes fix ahet.
17. Secrets plaintext files madhe (config/env/docs/tests/fixtures) — placeholders suddha nahi.
18. `gate::check` bypass karnara kuthlahi shortcut, cache, ki "trusted caller" exception.
19. Single-file thousands-of-lines modules; dead code; non-test `unwrap()` (Rust).

## 5. Jevha v2 madhe yapaiki kahitari have asel

`context.md` §6 madhe entry kara (date · change · karan) + PRD update kara — adhi nahi.
