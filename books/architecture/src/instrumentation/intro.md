# Instrumentation
Cuprate is built with [instrumentation](https://en.wikipedia.org/wiki/Instrumentation) in mind. 
The [tracing](https://docs.rs/tracing/latest/tracing/) crate is used to provide "structured, event-based diagnostic information".

As described in the tracing crate docs, there are 3 main concepts: spans, events and subscribers. Small explanations for
each will be included in the following chapters, however you should probably read the tracing docs.