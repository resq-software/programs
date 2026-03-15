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
name: state-inconsistency-auditor
description: Finds state inconsistency bugs where an operation mutates one piece of coupled state without updating its dependent counterpart, causing silent data corruption or reverts in subsequent operations. Triggers on /state-audit, state inconsistency audit, or coupled state audit.
---

# State Inconsistency Auditor

Finds structural bugs where coupled storage values (e.g., balance & checkpoint) get out of sync because a mutation path updates one but not the other.

## Core Methodology: Structural Invariant Analysis

1. **Map All Coupled State Pairs**: Identify variables that MUST stay in sync.
2. **Build Mutation Matrix**: List every function that modifies each variable.
3. **Cross-Check Mutations**: Identify gaps where one side of a pair is updated without the other.
4. **Ordering Check**: Verify that the order of updates doesn't create exploitable windows of inconsistency.
5. **Compare Parallel Paths**: Ensure similar operations (e.g., transfer vs burn) handle state consistently.
6. **Trace User Journeys**: Verify state remains consistent across multi-step interactions.

## Core Rules

RULE 0: MAP BEFORE YOU HUNT.
RULE 1: EVERY MUTATION PATH MATTERS.
RULE 2: PARTIAL OPERATIONS ARE THE #1 SOURCE OF BUGS.
RULE 3: COMPARE PARALLEL PATHS.
RULE 4: DEFENSIVE CODE (Ternary/Min/SafeMath) MASKS BUGS.
RULE 5: EVIDENCE-BASED FINDINGS ONLY.

## Verification Gate
Every finding must be verified via deep code trace or PoC test before reporting. Common false positives include hidden reconciliation hooks or intentional lazy evaluation.
