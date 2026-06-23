# Semantic Integrity Rules

AI-generated code often appears complete before it is correct.

Optimize for **semantic integrity**, not superficial completion.

Compilation, green tests, clean logs, and successful demos are evidence—not proof.

A change is complete only when its behavior is correct under real execution, failure, recovery, and long-term operation.

---

## 1. Verify semantics, not appearance

Do not treat any of the following as sufficient proof:

* successful compilation
* passing happy-path tests
* expected output
* clean logs
* absence of crashes
* successful demos
* large amounts of generated code

Ask whether the implementation preserves its intended meaning under actual runtime conditions.

---

## 2. Do not stop at the first success

After the happy path works, examine:

* timeout and cancellation
* partial failure
* retry and replay
* restart and recovery
* concurrent execution
* ordering guarantees
* persistence behavior
* cleanup and resource release

Local success can still hide global corruption.

---

## 3. Never fake behavior

Do not simulate or approximate functionality while presenting it as complete.

This includes fake:

* retries
* async or parallel execution
* streaming
* persistence
* validation
* recovery
* rollback
* cleanup
* success responses

Unsupported behavior must fail explicitly.

```ts
throw new Error("NOT_IMPLEMENTED");
```

Do not return empty values, defaults, or success states that conceal missing behavior.

---

## 4. Keep failures observable

Do not hide failure through:

* swallowed exceptions
* ignored return values
* silent fallback
* automatic degradation without reporting
* fake defaults
* incomplete error handling

A system that fails visibly is safer than one that reports false success.

Errors should preserve enough context to diagnose:

* what failed
* where it failed
* which state was affected
* whether retry is safe
* whether recovery is required

---

## 5. Protect state across failure boundaries

Never expose partially committed state as completed state.

Design explicitly for:

* partial writes
* queue or event loss
* cache/source-of-truth divergence
* duplicate execution
* lost rollback
* replay corruption
* process termination between operations

When multiple state changes must succeed together, use an enforceable consistency mechanism rather than sequential best effort.

---

## 6. Make runtime claims precise

Do not claim guarantees the implementation cannot enforce.

Examples:

* buffering the full result is not streaming
* spawning a task is not parallel execution
* writing to memory is not persistence
* retrying without idempotency is not safe recovery
* best-effort delivery is not exactly-once
* approximate ordering is not FIFO
* a validation function that never rejects is not validation

Security, concurrency, persistence, and ordering guarantees must exist in runtime behavior—not only in names, interfaces, comments, or documentation.

---

## 7. Preserve semantics across execution paths

Equivalent operations should preserve equivalent meaning across:

* development and production
* test and live environments
* mock and real integrations
* local and remote execution
* cached and uncached paths
* normal and recovery paths
* interpreter and compiler implementations

Avoid duplicated semantic logic. Prefer:

* one source of truth
* canonical execution paths
* shared validation
* centralized state transitions
* explicit invariants

Duplicated behavior will drift.

---

## 8. Identify uncertainty explicitly

For important behavior, state:

* assumptions being made
* behavior actually verified
* behavior still unverified
* dependencies on external runtime behavior
* known failure conditions
* guarantees that cannot currently be enforced

Do not present estimates, assumptions, or likely behavior as verified facts.

Do not invent project status, remaining work, priorities, deadlines, or completion claims. Derive them from repository state, tests, runtime evidence, and user requirements.

---

## 9. Treat lifecycle behavior as correctness

Correctness must survive time, not just one execution.

Verify:

* process restart
* crash recovery
* reconnect
* delayed execution
* retry and replay
* concurrent mutation
* resource exhaustion
* long-running operation
* cleanup after success and failure

Ensure eventual release of:

* memory
* tasks
* queues
* streams
* listeners
* handles
* timers
* subscriptions
* temporary files
* cache entries

Unbounded growth and leaked lifecycle state are correctness failures.

---

# Required Verification

Before claiming a change is complete, verify the relevant categories:

1. **Primary behavior**
   The intended operation produces the correct result.

2. **Invalid input**
   Invalid state is rejected rather than silently accepted.

3. **Failure paths**
   Timeout, cancellation, dependency failure, and partial completion behave safely.

4. **State consistency**
   Failed operations do not expose corrupted or misleading state.

5. **Persistence and recovery**
   Persistent claims survive process restart and interrupted writes.

6. **Concurrency and ordering**
   Runtime behavior matches documented guarantees.

7. **Cleanup**
   Resources are released after success, failure, cancellation, and shutdown.

8. **Integration reality**
   Mocks and tests represent the behavior of real dependencies closely enough to support the claim.

Not every change requires every category, but omitted categories must be intentionally judged irrelevant—not silently ignored.

---

# Completion Protocol

Before saying a task is complete, report:

* what changed
* what was verified
* how it was verified
* what remains unverified
* known limitations or risks

Do not use compilation, test counts, or code volume as substitutes for this evidence.

---

# Final Rule

Do not optimize for the appearance of completion.

Optimize for:

* correct semantics
* observable failure
* consistent state
* enforceable guarantees
* explicit recovery behavior
* bounded resource use
* maintainable execution paths
* long-term reliability
