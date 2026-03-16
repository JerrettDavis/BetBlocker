# BetBlocker Brand Guidelines

**Version:** 1.0
**Date:** 2026-03-15
**Status:** Active

---

## 1. Brand Name

**BetBlocker** — one word, camelCase with a capital B on both "Bet" and "Blocker."

- Correct: BetBlocker
- Incorrect: Bet Blocker, betblocker, BETBLOCKER, Bet-Blocker, BetBlocker.io

When referencing the project in code, use `betblocker` (all lowercase) for package names, domains, and container images. The display name in UI, documentation, and marketing is always **BetBlocker**.

---

## 2. Taglines

Use the tagline that best fits the context:

| Tagline | Best For |
|---------|----------|
| **Block the bet, not the bettor** | Marketing, social media, hero sections. Captures the philosophy that we target gambling access, not the person. |
| **Open-source gambling blocking for recovery** | Technical contexts, README headers, GitHub description. Communicates what the project is. |
| **Take back control** | Empowerment-focused messaging, onboarding, calls to action. Short and direct. |

---

## 3. Brand Values

These values are not decorative. Every design decision, feature choice, and communication should be traceable to one or more of these:

### Trust
People in recovery are trusting us with their protection at a vulnerable moment. We earn that trust through reliability, transparency, and honesty. We never overstate our capabilities. We never hide our limitations.

### Privacy
We don't spy. We don't sell data. We don't collect browsing history. We collect the minimum data required for blocking to work, and we tell users exactly what that data is. Privacy is not a feature — it is a constraint on every feature we build.

### Strength
Aggressive, tamper-resistant blocking is the core product promise. Half-measures help no one. We build protection that works even when the user's impulse to gamble is fighting against it. The blocking must be harder to defeat than the urge is to satisfy.

### Empowerment
We help people take control. We do not control them. Self-enrolled users can unenroll (with a cooling-off delay). We provide tools and information, not surveillance and punishment. The language we use always positions the user as the agent of their own recovery.

### Openness
Open source, transparent, community-driven. The code is public. The blocklist methodology is documented. The architecture decisions are explained. Anyone can audit, contribute, or self-host. Openness is how we earn trust at scale.

---

## 4. Voice and Tone

### Voice Characteristics

BetBlocker's voice is **professional but warm**. We sound like a knowledgeable friend who happens to be a security engineer — competent, calm, and genuinely caring.

| Characteristic | Description |
|---------------|-------------|
| **Clear** | We use plain language. When technical language is necessary, we explain it. No jargon for jargon's sake. |
| **Honest** | We say what the product does and does not do. We acknowledge limitations. We never use dark patterns or misleading language. |
| **Respectful** | We never moralize about gambling. We never use shame as a motivator. We treat every user as an adult making a brave decision. |
| **Empowering** | Our language positions the user as the one in control. "You chose to enroll," not "We enrolled you." "Your protection is active," not "We are protecting you." |
| **Precise** | When discussing security, privacy, or blocking capabilities, we are specific. "BetBlocker blocks DNS queries to known gambling domains" is better than "BetBlocker blocks gambling." |

### Tone Variations

The voice stays the same. The tone shifts by context:

**Onboarding and setup:** Warm, encouraging, patient. The user may be stressed or ashamed. Be calm.
- Good: "You're setting up protection for yourself. This takes about two minutes."
- Bad: "Let's fight your gambling addiction together!"

**Dashboard and status:** Neutral, factual, reassuring. Information density is high; keep language lean.
- Good: "3 blocked attempts today. All systems active."
- Bad: "Great job! You resisted 3 gambling temptations!"

**Technical documentation:** Clear, precise, structured. Developers and self-hosters need accuracy.
- Good: "The agent validates its own binary hash on startup using SHA-256."
- Bad: "Our super-secure agent checks itself for tampering."

**Error and alert states:** Direct, calm, actionable. No panic. Tell the user what happened and what to do.
- Good: "Connection to the BetBlocker server was lost. Blocking continues using your local blocklist. The agent will retry automatically."
- Bad: "WARNING: Server connection failed! Your protection may be compromised!"

**Marketing and landing pages:** Confident, values-driven, specific. Lead with what makes BetBlocker different.
- Good: "Free, open-source gambling blocking. No data collection. No subscription required."
- Bad: "The world's best gambling blocker!"

### Language Rules

**Always:**
- Use "gambling blocking" or "gambling block," not "addiction blocker"
- Say "enrolled" and "unenrolled," not "locked" and "unlocked"
- Use "protection" rather than "restriction"
- Refer to "accountability partner," not "supervisor" or "watcher"
- Use "recovery" when discussing the user's journey
- Use active voice: "BetBlocker blocks..." not "Gambling is blocked by..."

**Never:**
- Use gambling imagery or metaphors (dice, cards, chips, "jackpot," "all in")
- Moralize or preach about gambling behavior
- Use fear-based messaging ("You WILL relapse if...")
- Use infantilizing language ("We'll keep you safe")
- Make absolute claims about effectiveness ("100% protection")
- Use the word "addict" — prefer "person in recovery" or simply "user"

---

## 5. Color Palette

### Primary Colors

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **Indigo** | `#4F46E5` | 79, 70, 229 | Primary brand color. Trust, stability. Used for primary buttons, links, active states, logo. |
| **Indigo Light** | `#818CF8` | 129, 140, 248 | Hover states, secondary accents, light mode highlights. |
| **Indigo Dark** | `#3730A3` | 55, 48, 163 | Dark mode primary, pressed states, emphasis. |

### Secondary Colors

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **Slate** | `#475569` | 71, 85, 105 | Body text, secondary UI elements, professional neutrality. |
| **Slate Light** | `#94A3B8` | 148, 163, 184 | Muted text, borders, disabled states. |
| **Slate Dark** | `#1E293B` | 30, 41, 59 | Dark backgrounds, dark mode surfaces. |

### Semantic Colors

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **Emerald** | `#10B981` | 16, 185, 129 | Success states, recovery progress, "active" indicators. |
| **Amber** | `#F59E0B` | 245, 158, 11 | Warnings, alerts, attention needed. |
| **Rose** | `#F43F5E` | 244, 63, 94 | Danger, blocks, tamper detection, destructive actions. |

### Background and Surface

| Name | Hex | RGB | Usage |
|------|-----|-----|-------|
| **White** | `#FFFFFF` | 255, 255, 255 | Primary light background. |
| **Slate 50** | `#F8FAFC` | 248, 250, 252 | Secondary light background, cards, panels. |
| **Slate 900** | `#0F172A` | 15, 23, 42 | Primary dark mode background. |
| **Slate 800** | `#1E293B` | 30, 41, 59 | Secondary dark mode background, elevated surfaces. |

### Accessibility

All color combinations used for text must meet WCAG 2.1 AA contrast requirements:

| Combination | Contrast Ratio | Rating |
|-------------|---------------|--------|
| Indigo on White | 4.56:1 | AA (large text) |
| Indigo Dark on White | 7.82:1 | AAA |
| Slate on White | 5.39:1 | AA |
| White on Slate 900 | 15.39:1 | AAA |
| White on Indigo | 4.56:1 | AA (large text) |
| White on Indigo Dark | 7.82:1 | AAA |

For body text on light backgrounds, use Slate (`#475569`) or darker. For body text on dark backgrounds, use Slate 50 (`#F8FAFC`) or lighter. Never use Indigo alone for small body text on white — pair with Indigo Dark for adequate contrast.

---

## 6. Typography

### Primary Typeface: Inter

Used for all UI text, marketing copy, headings, body text, and labels.

- **Why:** Open-source, highly legible at all sizes, excellent for both UI and long-form reading. Designed for screens. Variable font support for fine-grained weight control.
- **Weights used:** 400 (Regular), 500 (Medium), 600 (SemiBold), 700 (Bold)
- **Fallback stack:** `'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`

### Code Typeface: JetBrains Mono

Used for code blocks, terminal output, technical identifiers, and the wordmark.

- **Why:** Open-source, excellent readability for code, distinctive ligatures, monospaced precision.
- **Weights used:** 400 (Regular), 700 (Bold)
- **Fallback stack:** `'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace`

### Type Scale

Based on a 1.25 ratio, rem units, with Inter:

| Level | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| Display | 3rem (48px) | 700 | 1.1 | Hero headlines |
| H1 | 2.25rem (36px) | 700 | 1.2 | Page titles |
| H2 | 1.75rem (28px) | 600 | 1.3 | Section headers |
| H3 | 1.375rem (22px) | 600 | 1.4 | Subsections |
| H4 | 1.125rem (18px) | 500 | 1.4 | Card titles, labels |
| Body | 1rem (16px) | 400 | 1.6 | Default text |
| Small | 0.875rem (14px) | 400 | 1.5 | Captions, metadata |
| Tiny | 0.75rem (12px) | 500 | 1.4 | Badges, fine print |

---

## 7. Logo Usage

### Logo Variants

The BetBlocker logo has three variants. See `logo.svg` for the source artwork.

1. **Full logo (icon + wordmark):** Primary usage. Used when there is sufficient horizontal space (minimum 200px wide).
2. **Icon only (shield):** Used for favicons, app icons, small UI placements, and social media avatars. Minimum size: 24x24px.
3. **Wordmark only:** Used in text-heavy contexts where the icon would feel redundant (e.g., alongside a large hero illustration that already includes the shield).

### Clear Space

Maintain clear space around the logo equal to the height of the "B" letterform in the wordmark. No other elements should intrude into this space.

### Minimum Sizes

| Variant | Minimum Width | Minimum Height |
|---------|--------------|----------------|
| Full logo | 200px | 40px |
| Icon only | 24px | 24px |
| Wordmark only | 120px | 20px |

### Do's

- Use the logo on white, Slate 50, Slate 900, or Indigo backgrounds
- Use the indigo version on light backgrounds
- Use the white version on dark or indigo backgrounds
- Maintain the prescribed clear space
- Link the logo to the homepage when used in navigation

### Don'ts

- Do not stretch, compress, or distort the logo
- Do not rotate the logo
- Do not add drop shadows, gradients, or effects
- Do not change the logo colors outside the approved palette
- Do not place the logo on busy or low-contrast backgrounds
- Do not recreate the logo in a different typeface
- Do not use the logo as a pattern or decorative element
- Do not animate the logo without brand team approval

---

## 8. Iconography and Imagery

### Icon Style

- Use outlined (stroke) icons, not filled, for UI elements
- Stroke width: 1.5px at 24px icon size, scale proportionally
- Corner radius: 2px for geometric icons
- Style: Lucide Icons or similar clean, geometric icon sets
- Color: Slate for inactive, Indigo for active/interactive

### Imagery Principles

- Use abstract, geometric, or architectural imagery — shields, grids, blocks, walls
- Prefer illustrations over photography when possible
- If photography is used: hands, nature, calm environments. Never faces (privacy metaphor).
- Never use gambling imagery: no dice, cards, chips, slot machines, roulette wheels, sports betting, casino interiors
- Never use imagery that could be triggering for someone in recovery

### Visual Metaphors

**Preferred:**
- Shield (protection)
- Wall / barrier (blocking)
- Lock (security)
- Checkpoint (control)
- Horizon / path (recovery journey)
- Open hand (empowerment, openness)

**Avoid:**
- Chains or cages (implies imprisonment, not empowerment)
- Crossed-out gambling items (still shows gambling imagery)
- Before/after recovery imagery (can be triggering)
- Dark, ominous imagery (we are about hope, not fear)

---

## 9. UI Component Patterns

### Buttons

| Type | Background | Text | Border | Usage |
|------|-----------|------|--------|-------|
| Primary | Indigo | White | None | Primary actions: "Enroll," "Save," "Confirm" |
| Secondary | White | Indigo | Indigo 1px | Secondary actions: "Cancel," "Back" |
| Danger | Rose | White | None | Destructive actions: "Unenroll," "Delete" |
| Ghost | Transparent | Slate | None | Tertiary actions, navigation links |

All buttons use Inter SemiBold (600), 14-16px, with 8px vertical and 16px horizontal padding. Border radius: 8px.

### Cards and Panels

- Background: White (light mode) or Slate 800 (dark mode)
- Border: 1px Slate 200 (light) or Slate 700 (dark)
- Border radius: 12px
- Padding: 24px
- Shadow: `0 1px 3px rgba(0, 0, 0, 0.1)` (light mode only)

### Status Indicators

| State | Color | Icon | Label Example |
|-------|-------|------|--------------|
| Active / Protected | Emerald | Shield check | "Protection active" |
| Warning / Attention | Amber | Alert triangle | "Update available" |
| Blocked / Danger | Rose | Shield X | "Tamper detected" |
| Inactive / Neutral | Slate Light | Circle | "Awaiting setup" |
| Syncing / Loading | Indigo | Spinner | "Syncing blocklist..." |

---

## 10. Brand Application Examples

### GitHub Repository

- **Description:** "Open-source gambling blocking for recovery"
- **Topics:** gambling-blocking, recovery, privacy, rust, open-source
- **Social preview:** Use `social-preview.html` screenshot
- **README header:** Full logo (icon + wordmark), tagline, badges

### Website

- **Navigation:** Icon logo (shield) + "BetBlocker" wordmark in Inter SemiBold
- **Hero:** "Take back control" headline, supporting copy, Enroll CTA in Indigo
- **Footer:** Full logo, "Block the bet, not the bettor" tagline, links

### Documentation

- **Header:** Icon logo, "BetBlocker Docs" in Inter SemiBold
- **Code blocks:** JetBrains Mono, Slate 900 background, Slate 50 text
- **Callouts:** Indigo border-left for info, Amber for warnings, Rose for danger

---

## 11. Brand Protection

### Trademark Considerations

- Register "BetBlocker" as a trademark in relevant jurisdictions
- The shield logo mark should be registered as a design mark
- Monitor for confusingly similar marks in the gambling and recovery software spaces

### Open Source and Brand

BetBlocker is open source. The code is freely available under its license. However:

- The BetBlocker name and logo are trademarks and are NOT covered by the software license
- Forks may use the codebase but must not use the BetBlocker name, logo, or branding
- Forks should be clearly distinguished from the official BetBlocker project
- Community contributions to the official project may use the brand under contributor guidelines

### Brand Monitoring

- Set up alerts for "BetBlocker" mentions across social media and news
- Monitor app stores for unauthorized uses of the name or logo
- Review community and third-party integrations for brand compliance
- Address brand misuse through the least restrictive means possible (education before legal)

---

## Appendix: CSS Custom Properties

```css
:root {
  /* Primary */
  --brand-indigo: #4F46E5;
  --brand-indigo-light: #818CF8;
  --brand-indigo-dark: #3730A3;

  /* Secondary */
  --brand-slate: #475569;
  --brand-slate-light: #94A3B8;
  --brand-slate-dark: #1E293B;

  /* Semantic */
  --brand-emerald: #10B981;
  --brand-amber: #F59E0B;
  --brand-rose: #F43F5E;

  /* Backgrounds */
  --brand-bg-light: #FFFFFF;
  --brand-bg-light-alt: #F8FAFC;
  --brand-bg-dark: #0F172A;
  --brand-bg-dark-alt: #1E293B;

  /* Typography */
  --brand-font-ui: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --brand-font-code: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace;

  /* Spacing */
  --brand-space-xs: 0.25rem;
  --brand-space-sm: 0.5rem;
  --brand-space-md: 1rem;
  --brand-space-lg: 1.5rem;
  --brand-space-xl: 2rem;
  --brand-space-2xl: 3rem;
  --brand-space-3xl: 4rem;

  /* Radii */
  --brand-radius-sm: 4px;
  --brand-radius-md: 8px;
  --brand-radius-lg: 12px;
  --brand-radius-xl: 16px;
  --brand-radius-full: 9999px;

  /* Shadows (light mode) */
  --brand-shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
  --brand-shadow-md: 0 1px 3px rgba(0, 0, 0, 0.1);
  --brand-shadow-lg: 0 4px 6px rgba(0, 0, 0, 0.1);
}
```
