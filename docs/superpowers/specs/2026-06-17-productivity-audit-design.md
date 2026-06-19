# Design: Phase 1 - The Productivity-Spending Correlation Audit

## Overview
This audit aims to establish a "Source of Truth" by correlating financial spending patterns with professional productivity (GitHub activity). The goal is to identify the relationship between AI/Impulse spending and coding output to inform future optimization strategies.

## 1. Data Ingestion & Cleaning
### 1.1 Financial Data
- **Sources:** `download-transactions(2).csv` and provided text for pending transactions.
- **Normalization:** All transactions will be converted to **CAD** using the exchange rates provided in the CSV.
- **Cleaning:** Remove noise from merchant descriptions to facilitate pattern matching.

### 1.2 Productivity Data
- **Source:** GitHub CLI (`gh`).
- **Metrics:** Commits, push events, and pull request activity for the period Jan 2026 - June 2026.

## 2. Targeted Categorization
Transactions will be mapped into four primary segments:
1.  **AI_SPEND:** Subscriptions and API usage for AI services (OpenAI, Claude, OpenRouter, DeepSeek, etc.).
2.  **WEED_IMPULSE_SPEND:** Transactions identified as cannabis-related or high-frequency convenience spending (e.g., UberEats, fast food, bars).
3.  **ESSENTIAL:** Necessary living expenses (Rent, basic groceries, utilities, essential transport).
4.  **NON_ESSENTIAL_OTHER:** All other discretionary spending.

## 3. Correlation & Analysis
The analysis will compute:
- **AI ROI:** `Total AI Spend / Total GitHub Activity`.
- **Behavioral Correlation:** Statistical correlation between `Daily Spending (AI + Weed/Impulse)` and `Daily GitHub Activity`.
- **The "Cost of Inactivity":** Daily burn rate of subscriptions during periods of zero GitHub activity.

## 4. Visualizations
- **The Correlation Plot:** A dual-axis time-series graph (Spending vs. Commits).
- **The Segmented Sunburst:** A radial chart showing the distribution of spending across the four segments.
- **The Leakage Heatmap:** A calendar view of high-spending days.

## 5. Constraints & Privacy
- **Privacy:** No data will be transmitted outside the local environment.
- **Integrity:** All calculations must use the normalized CAD values.
