# Shutdown
Once the read/write handles to the `tower::Service` are `Drop`ed, the backing thread(pool) will gracefully exit, automatically.

Note the writer thread and reader threadpool aren't connected whatsoever; dropping the write handle will make the writer thread exit, however, the reader handle is free to be held onto and can be continued to be read from - and vice-versa for the write handle.
