# rx_rust_mp
Message Passing implementation prototype of the [ReactiveX API](https://reactivex.io/)

This is a protoype only implementing the operators I needed for my master thesis.
I created it after discovering that the [official implementation](https://github.com/ReactiveX/RxRust) hasn't been updated for 8 years,
and the unofficial [rxRust](https://github.com/rxRust/rxRust) uses a shared memory model internally, making parallel computation of
stream data quasi-impossible.

The library itself is pretty simple, there is one trait [`Observable`](https://github.com/AntonOellerer/rx_rust_mp/blob/main/src/observable.rs),
which provides the implementations creating each operator, and requires implementing structs to implement the [`actual_subscribe`](https://github.com/AntonOellerer/rx_rust_mp/blob/main/src/observable.rs)
function. Due to this, every struct implementing `Observable` can be chained into a stream.  
At the end of the stream declaration [`subscribe`](https://github.com/AntonOellerer/rx_rust_mp/blob/main/src/observable.rs#L123) has to be called,
being given a function to execute on each incoming value, and a pool to schedule each task on.  
This `subscribe` function calls the `actual_subscribe` of the operator above it, handing it the pool and the `Sender` part of a mpsc channel,
which is repeated for each operator until the [`create`](https://github.com/AntonOellerer/rx_rust_mp/blob/main/src/create.rs)
or [`from_iter`](https://github.com/AntonOellerer/rx_rust_mp/blob/main/src/from_iter.rs) function at the top of the stream declaration is reached.

Each operator needs to at least store a reference to the struct above, so that it can refer to it once the stream is constructed on `subscribe`.
The general workflow of each operators `actual_subscribe` function is 
 1. creating a mpsc channel, 
 2. scheduling a thread on the thread pool which 
    1. reads from the receiver end of the channel created in `(1)` 
    2. executes the required transformations on each incoming value
    3. sends the result down the channel passed to the `actual_subscribe` function
3. invoking the `actual_subscribe` function of the previous object, passing it the sending end of the channel created in `(1)` and the thread pool

This is of course not a strict recipe, as each operator has to do different things.
