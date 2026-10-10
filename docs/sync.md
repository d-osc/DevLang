# Shared channels, mutexes and cancellation

`use "std/sync"` provides runtime primitives that share their state across `spawn`, `async fn` and captured closures. Copying a handle retains the same resource through reference counting. This differs from ordinary Vec/Map/struct value copies. These APIs currently run in the interpreter, not native builds; workers are OS threads rather than coroutines.

## Channel<T>

Create a bounded FIFO queue with `sync.channel<T>(capacity)`. Specify `T` explicitly; capacity must be 1–65536. Multiple producers and consumers may share it. Queued values preserve ordinary value semantics; mutation of a received value does not modify the sender's copy.

| API | Behavior |
|---|---|
| `send(value) bool` | Wait for capacity; false if closed |
| `trySend(value) bool` | True when enqueued; false if full or closed |
| `sendTimeout(value, ms) bool` | Wait up to the queue-wait deadline; false if full at timeout or closed |
| `receive() Receive<T>` | Wait for an item or channel closure |
| `tryReceive() Receive<T>` | Read without waiting for an item |
| `receiveTimeout(ms) Receive<T>` | Wait up to the queue-wait deadline |
| `close() bool` | Prevent sends and wake waiters; true only for the first close |
| `isClosed() bool` | Whether sends are disabled |
| `len() i64`, `capacity() i64` | Current queued count and item capacity |

`Receive<T>` has `Item(T)`, `Empty`, and `Closed`. Empty means a poll/timeout found no item. Closing preserves queued values for draining; Closed is returned after they have been consumed. A sender may have returned true before another thread closes the queue. There is no fairness guarantee among waiting threads and no rendezvous/unbounded queue mode.

```dev
use "std/sync"
fn main() {
    let queue = sync.channel<i64>(1)
    let producer = spawn(fn() { queue.send(42); queue.close() })
    match queue.receive() {
        Item(value) => { print(value) }
        Empty => {}
        Closed => {}
    }
    await(producer)
}
main()
```

Always arrange for another thread to consume a blocking send, supply a timeout, or close the channel. A handle being dropped does not close a queue automatically: this API has a combined send/receive handle rather than separate endpoint ownership. A worker holding the last handle can otherwise wait indefinitely.

## Mutex<T>

`sync.mutex(value)` infers T. `get()` returns a protected value snapshot. `set(value)` replaces it. `update(fn(T) T)` runs the callback with exclusive access and commits its returned value; it also returns the new value. `updateTimeout(callback, ms)` limits waiting to acquire exclusive access, not execution time of the callback. `close()` releases the stored value and disables access; repeated closes return false.

```dev
use "std/sync"
fn main() {
    let counter = sync.mutex(0)
    let worker = spawn(fn() {
        for i in 0..1000 { counter.update(fn(n i64) i64 { return n + 1 }) }
    })
    for i in 0..1000 { counter.update(fn(n i64) i64 { return n + 1 }) }
    await(worker)
    print(counter.get())
    counter.close()
}
main()
```

The callback lease is released on success, runtime error, or Rust unwinding. A callback error leaves the stored value unchanged; side effects outside the mutex are not rolled back. Calling get/set/update/close on that same mutex from its callback raises a reentry error instead of deadlocking. Calling another mutex can still create a lock-order deadlock, including one involving multiple workers; use a consistent order and timed acquisition. Awaiting a worker that needs the held mutex can also deadlock, so avoid blocking dependencies inside update callbacks. There is no manual lock/unlock or mutable guard to forget to release. `get(); set(old+1)` is two operations and is not an atomic increment: use `update`.

## CancelToken

`sync.token()` creates a one-way cancellation flag. Copies share the flag. `cancel()` returns true for the first cancellation, false thereafter, and wakes token waiters. `isCancelled()` polls; `check()` raises a catchable `operation cancelled` error. `wait(ms)` waits for cancellation and returns true, or returns false at its wait deadline. A cancelled token cannot be reset.

```dev
use "std/sync"
fn main() {
    let token = sync.token()
    let worker = spawn(fn() i64 {
        while !token.wait(1) { /* perform one bounded unit of work */ }
        return 42
    })
    token.cancel()
    print(await(worker))
}
main()
```

Cancellation is cooperative: it does not forcibly terminate a Task, interrupt a channel/mutex/network wait, undo writes, or detach/join a worker. For a cancellable channel consumer, use receiveTimeout and check the token between waits; close the channel to wake ordinary blocked consumers/senders. Await workers before process exit.

## Data and resource limits

Channels/mutexes accept scalar data, structs, payload enums, Ref, Vec, Map, Slice and JSON values. Pointers, function values, Tasks and sync handles are rejected anywhere inside stored payloads. This prevents shared-resource/closure cycles in these containers and avoids implicitly transferring executable or raw pointer ownership. Other runtime module handles are ordinary records: copying their record through a channel does not transfer the underlying interpreter-owned resource.

Each channel has an 8 MiB estimated queued-model budget, in addition to item capacity. A single mutex value has the same budget. The estimate includes 64 bytes per model node plus string/key bytes; it is not a hard process RAM cap, and repeated shared references count repeatedly. Depth is capped at 128. Payload validation or allocation failures raise catchable errors. Full byte capacity behaves like full item capacity.

An interpreter may create at most 64 live sync resources, also subject to its global 256-resource cap. Creation tracking uses weak references: resources are reclaimed when the last copied/captured handle is released. Closed but still referenced handles count toward the limit. Imported/captured handles do not become new creations in another worker. There is no process-wide thread/resource quota.

All timeout values are integer milliseconds in 0–300000; zero polls/acquires if immediately available. Deadlines use a monotonic clock and predicate loops tolerate spurious wakes. These are best-effort wait deadlines: scheduling, payload validation and metadata locking may add delay. Try operations avoid waiting for queue capacity/items, but still briefly acquire the metadata mutex.

See [the runnable example](../examples/sync/main.dev) and `scripts/smoke_sync.py` for multi-producer/consumer stress, contended atomic updates, callback recovery, close wakeups, value snapshots, budgets and real cancellation waits.
