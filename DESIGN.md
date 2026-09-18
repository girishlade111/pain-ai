# pain ai — DESIGN.md (Claude-Desktop-grade Design System + App Mapping)

**Rule:** app chi design same-to-same Claude Desktop sarkhi disali pahije. Khali tokens + components + app mapping. Builders: `{token.refs}` everywhere — hex inline nahi (fakt `tokens.ts` madhech hex).

> NOTE on fonts: Copernicus / StyreneB are licensed Anthropic faces. App uses open substitutes — **Cormorant Garamond 500** (display serif) + **Inter** (body sans) + **JetBrains Mono** (code). Weight/tracking rules unchanged.

---

## 1. Overview

Warmest AI-product interface. Base = **tinted cream canvas** (`{colors.canvas}` #faf9f5) — warm, not cool gray-white. Headlines = **slab-serif display** (Cormorant Garamond, weight 400/500, negative tracking); body = **Inter**. Brand voltage = **cream + coral** (`{colors.primary}` #cc785c) — warm muted coral, never cyan/blue. Three surface modes alternate: **cream canvas** (default) → **light cream cards** (`{colors.surface-card}`) → **dark navy product surfaces** (`{colors.surface-dark}`) where the agent shows real work (code, diffs, terminal, approval details). Cream-to-dark contrast = page pacing rhythm.

## 2. Colors

### Brand & Accent
| Token | Hex | Use |
|---|---|---|
| `{colors.primary}` | #cc785c | Signature coral. Primary CTAs (Send, Approve, Install), full-bleed callout cards, wordmark accent |
| `{colors.primary-active}` | #a9583e | Press/hover-darker |
| `{colors.primary-disabled}` | #e6dfd8 | Desaturated disabled |
| `{colors.accent-teal}` | #5db8a6 | Secondary status: connected dots, active-connection indicators |
| `{colors.accent-amber}` | #e8a55a | Category badges, inline highlights, Medium-risk pill |

### Surface
| Token | Hex | Use |
|---|---|---|
| `{colors.canvas}` | #faf9f5 | Default app floor (main window, chat background) |
| `{colors.surface-soft}` | #f5f0e8 | Sidebar, section dividers, soft bands |
| `{colors.surface-card}` | #efe9de | Feature/settings/skill cards, approval card body |
| `{colors.surface-cream-strong}` | #e8e0d2 | Selected tabs, emphasized bands |
| `{colors.surface-dark}` | #181715 | Code/diff/terminal panels, model reply chrome, footer, chibi settings panel |
| `{colors.surface-dark-elevated}` | #252320 | Elevated panels inside dark (secondary buttons on dark, nested cards) |
| `{colors.surface-dark-soft}` | #1f1e1b | Inner code blocks inside dark cards |
| `{colors.hairline}` | #e6dfd8 | 1px borders on cream |
| `{colors.hairline-soft}` | #ebe6df | Barely-visible inner dividers |

### Text
| Token | Hex | Use |
|---|---|---|
| `{colors.ink}` | #141413 | Headlines, primary text (warm off-black) |
| `{colors.body-strong}` | #252523 | Emphasized paragraphs, lead text |
| `{colors.body}` | #3d3d3a | Default running text |
| `{colors.muted}` | #6c6a64 | Sub-heads, breadcrumbs, secondary |
| `{colors.muted-soft}` | #8e8b82 | Captions, fine print, line numbers |
| `{colors.on-primary}` | #ffffff | Text on coral |
| `{colors.on-dark}` | #faf9f5 | Cream-white on dark |
| `{colors.on-dark-soft}` | #a09d96 | Footer/secondary on dark, TTS caption secondary |

### Semantic
| Token | Hex | Use |
|---|---|---|
| `{colors.success}` | #5db872 | Available/healthy dots |
| `{colors.warning}` | #d4a017 | Warning callouts (rare) |
| `{colors.error}` | #c64545 | Validation errors, Deny confirmations |

## 3. Typography

Family: display `Cormorant Garamond, Tiempos Headline, Garamond, "Times New Roman", serif`; body `Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`; code `JetBrains Mono, monospace`.

| Token | Size | Wt | LH | Track | Use |
|---|---|---|---|---|---|
| `{typography.display-xl}` | 64px | 400 | 1.05 | -1.5px | Welcome/empty-state h1 (serif) |
| `{typography.display-lg}` | 48px | 400 | 1.1 | -1px | Section heads (serif) |
| `{typography.display-md}` | 36px | 400 | 1.15 | -0.5px | Sub-section heads (serif) |
| `{typography.display-sm}` | 28px | 400 | 1.2 | -0.3px | Settings page titles, callout heads (serif) |
| `{typography.title-lg}` | 22px | 500 | 1.3 | 0 | Plan/price-size labels |
| `{typography.title-md}` | 18px | 500 | 1.4 | 0 | Card titles, intro paragraphs |
| `{typography.title-sm}` | 16px | 500 | 1.4 | 0 | Connector tiles, list labels |
| `{typography.body-md}` | 16px | 400 | 1.55 | 0 | Default running text |
| `{typography.body-sm}` | 14px | 400 | 1.55 | 0 | Secondary body, fine print |
| `{typography.caption}` | 13px | 500 | 1.4 | 0 | Badges, captions |
| `{typography.caption-uppercase}` | 12px | 500 | 1.4 | 1.5px | Category tags, NEW/BETA |
| `{typography.code}` | 14px | 400 | 1.6 | 0 | Code/diff/terminal |
| `{typography.button}` | 14px | 500 | 1.0 | 0 | Button labels |
| `{typography.nav-link}` | 14px | 500 | 1.4 | 0 | Sidebar nav items |

Principles: display never bold (400 only); negative tracking non-negotiable; body 400 / labels 500; bigger-serif-before-bolder for emphasis.

## 4. Layout & Spacing

Base 4px. `{spacing.xxs}` 4 · `{spacing.xs}` 8 · `{spacing.sm}` 12 · `{spacing.md}` 16 · `{spacing.lg}` 24 · `{spacing.xl}` 32 · `{spacing.xxl}` 48 · `{spacing.section}` 96.
- App max content ~1200px centered; chat column max ~820px.
- Card internal padding `{spacing.xl}` 32 (feature/settings/skill cards); code/connector tiles `{spacing.lg}` 24.
- Section gaps 96 desktop / 48 mobile. Window chrome: sidebar 264px desktop, overlay sheet mobile.

## 5. Elevation & Shapes

Color-block first, shadow rare (hover-only `0 1px 3px rgba(20,20,19,0.08)`). Depth from cream-vs-dark contrast. Dark panels carry own chrome (line numbers `{colors.muted-soft}`, syntax colors, status bars `{colors.surface-dark-elevated}`).
Radius: `{rounded.xs}` 4 (tiny dropdowns) · `{rounded.sm}` 6 (inline btns) · `{rounded.md}` 8 (CTAs, inputs, tabs) · `{rounded.lg}` 12 (cards, code windows, approval cards) · `{rounded.xl}` 16 (hero/empty-state art, chibi stage) · `{rounded.pill}` 9999 (badges) · `{rounded.full}` 50% (avatars, icon buttons).
Spike-mark (4-spoke radial asterisk) = inline SVG logo asset, brand wordmark prefix.

## 6. App Component Library (Claude Desktop mapping — build THESE)

### Shell
- **`app-window`**: canvas floor; sidebar surface-soft 264px (logo + spike-mark, nav: Chat / Skills / Connectors / Cron / Memory / Settings, `{typography.nav-link}`); main chat column; right inspector (closed by default).
- **`chat-bubble-user`**: surface-card bg, ink text, radius lg, max 85% width, right-aligned.
- **`chat-bubble-agent`**: canvas bg, body text; serif display-sm for its headline lines; model chrome (code/diff/terminal) inside `code-window-card`.
- **`composer`**: canvas input, hairline border, radius md, h 40+ autogrow; focus = coral border + 3px coral-15% ring; mic button (voice) + Send `button-primary`.
- **`empty-state`**: display-xl serif welcome + line-art illustration card (`rounded.xl`) + 3 suggestion `feature-cards`.

### Buttons / Inputs / Badges
- **`button-primary`**: coral bg, white text, 14/500, pad 12×20, h 40, radius md; active `#a9583e`; disabled `#e6dfd8`.
- **`button-secondary`**: canvas bg, ink text, 1px hairline; same metrics.
- **`button-secondary-on-dark`**: `#252320` bg, on-dark text (never invert to light).
- **`button-text-link`**: no bg, coral text; underline on press.
- **`button-icon-circular`**: 36px circle, canvas bg, hairline border.
- **`text-input`**: canvas bg, ink, body-md, radius md, pad 10×14, h 40, hairline border; focus per composer.
- **`badge-pill`**: surface-card bg, ink, caption 13/500, pill, pad 4×12. **`badge-coral`**: coral bg white text, 12/500/1.5px tracking (NEW/BETA/High-risk). **`badge-risk-med`**: amber; **`badge-risk-low`**: teal.
- **`category-tab` / `-active`**: inactive transparent muted; active surface-card ink, pad 8×14 radius md.

### Work cards
- **`feature-card`**: surface-card, radius lg, pad 32; icon top + title-md + body-md. (Suggestion cards, onboarding, memory highlights.)
- **`code-window-card`**: surface-dark, inner block surface-dark-soft, JetBrains Mono, line numbers muted-soft, pad 24, radius lg; horizontal scroll on narrow (never wrap code).
- **`approval-card`** (most important custom component): surface-card body radius lg pad 32; title-md action summary; risk pill top-right; detail block = `code-window-card` mini (exact command / file diff / UI target + app name); "why" line body-sm muted; 4 buttons row: Allow Once (secondary) / Workspace (secondary) / Global (secondary) / Deny (text-link error); Tab opens comment field. Voice approvals speak the summary line AND show this card.
- **`caption-bar`**: below composer, on-dark-soft secondary text; active spoken sentence in ink; follows TTS queue position.
- **`trust-dialog`**: modal cream card radius lg; lists rules/dirs the folder would activate; Accept (primary) / Continue-without (secondary).
- **`connector-tile`**: canvas + hairline, radius lg, pad 20; logo + title-sm + desc; status dot (teal/amber/muted); enable switch.
- **`cron-row` / `memory-row`**: list rows, hairline-soft dividers, caption-uppercase schedule tags.
- **`chibi-stage`**: transparent always-on-top window; sprite card radius xl; settings popover dark.
- **`callout-card-coral`**: full-bleed coral, white text, radius lg, pad 48 — major moments only (first-run success, update notes). CTA inside = inverted cream button.
- **`footer-bar`**: surface-dark strip, on-dark-soft text (sync status, model name, gate mode).

## 7. Do's and Don'ts (binding)

Do: cream canvas always; serif display 400 + negative tracking; coral scarce on buttons, generous only on full-bleed callouts; alternate cream → cream-card → dark-mockup bands; real product chrome over illustrations; 96px section rhythm; bigger-serif-before-bolder.
Don't: pure white / cool grays; bold serif; blue/cyan accents; coral everywhere; sans display headlines; same surface twice in a row; undocumented hover states.

## 8. Responsive

| Breakpoint | Behavior |
|---|---|
| <768px | Sidebar → hamburger cream sheet; chat full-width; h1 64→32; grids 1-up; code cards h-scroll |
| 768–1024px | Tight sidebar icons+labels; cards 2-up |
| 1024–1440px | Full 264px sidebar; 3-up cards |
| >1440px | Same, content capped 1200px |

Touch targets ≥40px (icon-circular 36 visual, 44 hit-slop). Connector tiles fully tappable.

## 9. Iteration Guide (for prompt builders)

1. ONE component per task; cite its `{component.key}`.
2. Variants (`-active/-disabled/-focused`) = separate entries.
3. `{token.refs}` everywhere; hex only in `tokens.ts`.
4. Default + Active/Pressed states only — never document hover.
5. Trinity check: cream + coral + dark navy. Fourth tone = reject.
6. Emphasis ladder: size → serif → weight (last resort).
