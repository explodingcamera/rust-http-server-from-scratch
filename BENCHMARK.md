```bash
$ wrk -t6 -c200 -d10s http://localhost:8080
```

# Hello World response, no request parsing

```
Running 10s test @ http://localhost:8080
  6 threads and 200 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     3.44ms   27.80ms 824.11ms   98.56%
    Req/Sec    24.79k     3.48k   38.16k    79.50%
  1480309 requests in 10.01s, 160.94MB read
  Socket errors: connect 0, read 843703, write 636604, timeout 0
Requests/sec: 147920.21
Transfer/sec:     16.08MB
```
