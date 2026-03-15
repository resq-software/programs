<!--
  Copyright 2026 ResQ

  Licensed under the Apache License, Version 2.0 (the "License");
  you may not use this file except in compliance with the License.
  You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing, software
  distributed under the License is distributed on an "AS IS" BASIS,
  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  See the License for the specific language governing permissions and
  limitations under the License.
-->

---
name: nemesis-auditor
description: "The Inescapable Auditor. Runs the full Feynman Auditor (Stage 1) and full State Inconsistency Auditor (Stage 2) as primary steps, then fuses their outputs in a feedback loop (Stage 3) to find bugs at the intersection that neither alone would catch. Language-agnostic. Triggers on /nemesis or nemesis audit."
---

# N E M E S I S

### The Inescapable Auditor

Nemesis is an **iterative back-and-forth loop** where Feynman and State Inconsistency run alternating passes until no new bugs surface.

**Pass 1 (Feynman)** — Every line questioned, ordering challenged, assumptions exposed.
**Pass 2 (State)** — Enriched by Pass 1 results. Maps every coupled state pair and identifies gaps.
**Pass 3 (Feynman)** — Targeted re-interrogation of state gaps and masking code.
**Pass 4 (State)** — Targeted re-analysis of new coupled pairs and root causes.

The loop continues until convergence (no new findings).

---

## The Nemesis Execution Model

Nemesis uses a tiered pipeline:
1. **RECON**: Attacker mindset and initial coupling hypothesis.
2. **FOUNDATION**: Building the Function-State Matrix and Coupled State Dependency Map.
3. **HUNT PASSES**: Alternating Feynman Interrogation and State Cross-Checks.
4. **FEEDBACK LOOP**: The core innovation where auditors interrogate each other's findings.
5. **SEQUENCES**: Multi-transaction journey tracing for confirmed bugs.
6. **VERIFY**: Mandatory verification via code trace or PoC.
7. **DELIVER**: Final report of verified true positives.

---

## Core Rules

RULE 0: THE ITERATIVE LOOP IS MANDATORY.
RULE 1: FULL FIRST, TARGETED AFTER.
RULE 2: EVERY COUPLED PAIR GETS INTERROGATED.
RULE 3: EVERY FEYNMAN SUSPECT GETS STATE-TRACED.
RULE 4: PARTIAL OPERATIONS + ORDERING = GOLD.
RULE 5: DEFENSIVE CODE IS A SIGNAL, NOT A SOLUTION.
RULE 6: EVIDENCE OR SILENCE.
