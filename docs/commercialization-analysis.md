# Claude Tabs Commercialization Analysis

## Product Overview

Claude Tabs is a tab-based terminal manager for Claude Code power users, featuring session management, history search, and cloud sync.

---

## Pricing Model

| Plan | Price | Annual Discount |
|------|-------|-----------------|
| Monthly | $8/month | — |
| Yearly | $68/year | 29% off |
| Trial | 14 days | Full features |

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Payment processor | Stripe | Lower fees at scale, avoid MoR lock-in |
| Auth provider | Clerk | Best DX for desktop + web, Stripe integration |
| Database | Turso | SQLite-compatible, edge-distributed, per-user DBs |
| Offline support | None | Reduces complexity significantly |
| Device limit | Unlimited | No device management overhead |
| Data retention | 1 year | After subscription lapses |
| Geographic scope | US-only initially | Simplifies tax compliance |

---

## Revenue Model

### Per-Transaction Economics

| Plan | Price | Stripe Fee | Net | Fee % |
|------|-------|------------|-----|-------|
| Monthly | $8.00 | $0.53 | $7.47 | 6.6% |
| Yearly | $68.00 | $2.27 | $65.73 | 3.3% |

### Projected Revenue (60% monthly / 40% yearly mix)

| Users | Gross/mo | Net/mo | Annual Net |
|-------|----------|--------|------------|
| 100 | $707 | $660 | $7,920 |
| 500 | $3,533 | $3,310 | $39,720 |
| 1,000 | $7,067 | $6,643 | $79,716 |
| 5,000 | $35,333 | $33,333 | $400,000 |
| 10,000 | $70,667 | $66,693 | $800,316 |

---

## Cost Structure

### Infrastructure Costs by Scale

| Service | Free Tier Limit | Paid Tier |
|---------|-----------------|-----------|
| Turso | 500 DBs, 9GB | $29/mo (10K DBs) |
| Clerk | 10K MAU | $0.02/MAU after |
| Cloudflare Workers | 100K req/day | $5/mo |
| Vercel | 100GB bandwidth | $20/mo |
| Domain | — | $12/year |

### Total Infrastructure Costs

| Scale | Monthly Cost |
|-------|--------------|
| 0-500 users | $1 |
| 1,000 users | $30 |
| 5,000 users | $30 |
| 10,000 users | $35 |

### One-Time Setup Costs

| Item | Cost |
|------|------|
| Domain registration | $12 |
| Apple Developer account | $99/year |
| **Total to launch** | **~$111** |

---

## Margin Analysis

| Users | Gross | Stripe Fees | Infrastructure | Net | Margin |
|-------|-------|-------------|----------------|-----|--------|
| 1,000 | $7,067 | $394 | $30 | $6,643 | 94.0% |
| 5,000 | $35,333 | $1,970 | $30 | $33,333 | 94.3% |
| 10,000 | $70,667 | $3,940 | $35 | $66,693 | 94.4% |

---

## Break-Even Analysis

- **Fixed costs**: ~$30/month (Turso + domain)
- **Break-even point**: 4-5 paying users

---

## Comparison: Stripe vs Merchant of Record

| Metric | Stripe | LemonSqueezy (5%) |
|--------|--------|-------------------|
| Fee at $8/mo | 6.6% | 11.25% |
| Fee at $68/yr | 3.3% | 5.7% |
| 1K users cost | $394/mo | $550/mo |
| 10K users cost | $3,940/mo | $5,500/mo |
| Tax compliance | You handle (US only) | They handle (global) |
| Migration risk | Low (you own it) | High (locked in) |

**Annual savings with Stripe at 10K users: ~$18,720**

---

## Target Market

- **Primary**: Claude Code power users
- **Characteristics**:
  - Already paying $20+/mo for Claude Pro or API
  - Developers with high willingness to pay
  - Value productivity tools
  - Primarily US-based

---

## Risk Factors

| Risk | Mitigation |
|------|------------|
| US-only limits market | Add international when revenue justifies tax compliance |
| Internet required | Target market has reliable connectivity |
| Turso dependency | Standard SQLite, can migrate if needed |
| Clerk dependency | OAuth is standardized, can swap providers |

---

## Success Metrics

| Milestone | Users | Monthly Revenue |
|-----------|-------|-----------------|
| Ramen profitable | 100 | $660 |
| Part-time income | 500 | $3,310 |
| Full-time income | 1,000 | $6,643 |
| Small team | 5,000 | $33,333 |
| Serious business | 10,000 | $66,693 |
