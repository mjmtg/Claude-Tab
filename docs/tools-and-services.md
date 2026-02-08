# Tools and Services to Set Up

## Payment & Billing

- [ ] **Stripe** — Payment processing
  - Account setup
  - Stripe Billing (subscriptions)
  - Stripe Tax (US sales tax)
  - Customer Portal (self-service billing management)
  - Webhook endpoints

## Authentication

- [ ] **Clerk** — User authentication
  - Account setup
  - OAuth configuration for desktop app
  - Stripe integration (link users to customers)
  - JWT configuration

## Database

- [ ] **Turso** — Cloud database
  - Account setup
  - Database provisioning strategy (one DB per user)
  - Platform API access for DB creation

## Backend Hosting

- [ ] **Cloudflare Workers** — API hosting
  - Account setup
  - Worker deployment
  - Environment variables / secrets

## Website Hosting

- [ ] **Vercel** — Static site hosting
  - Account setup
  - Domain configuration
  - Deployment pipeline

## Domain

- [ ] **Domain registrar** (Namecheap, Cloudflare, etc.)
  - Purchase domain
  - DNS configuration
  - Email forwarding (optional)

## App Distribution

- [ ] **Apple Developer Account** — Mac app distribution
  - Account setup ($99/year)
  - Code signing certificates
  - Notarization setup

## Development Tools

- [ ] **GitHub** (or existing repo host)
  - Website repository
  - CI/CD for deployments

---

## Service Dependencies

```
Stripe ──────────► Webhooks ──────────► Cloudflare Workers
                                              │
Clerk ───────────► JWT Validation ────────────┤
                                              │
                                              ▼
                                        Turso (DB provisioning)
```

---

## Account Checklist

| Service | URL | Free Tier |
|---------|-----|-----------|
| Stripe | stripe.com | Yes (pay per transaction) |
| Clerk | clerk.com | 10K MAU |
| Turso | turso.tech | 500 DBs |
| Cloudflare | cloudflare.com | 100K req/day |
| Vercel | vercel.com | Hobby tier |
| Apple Developer | developer.apple.com | No ($99/year) |
