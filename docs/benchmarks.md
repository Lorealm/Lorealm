# Benchmarks

Lorealm can support thousands of users. This has been proven by the following benchmarks. You
can run them yourself with the `bench` tool.

## Setup
The numbers below come from a local setup on my home machine with the following key specifications:
```
CPU: Ryzen 7 7800X3D
RAM: 64 GB DDR5 (6400MHz)
OS: Arch Linux (7.2.11-arch1-1)
```

The configured storage backend was a Postgres database and a Redis cache running on the same
machine as the Lorealm server. Proper hosted scenarios would use a dedicated machine for the
storage backend.

## Scenario
The scenario we're running against is on the heavier side of a realistic workload. It assumes
every single user has three concurrent conversations streaming. Most users only use one. It
has a base thinking timeout of 30s, which can vary with 50% in either direction. This emulates
the time spent reading the message or doing other things. Often a user may be doing something
that takes less load, such as editing a character. The benchmark's load generator was also
running on the same machine, which contributes to ~17% of the CPU overhead.

Only conversations were measured in the benchmark below, as conversations are virtually the only
load-bearing part of Lorealm, comparatively speaking.

## Results
> Nov. 21, 2026. Against Lorealm v0.2.0.

> Overview: 15000 concurrent conversations, which equals 5000 active users.

```
── step 1: 15000 concurrent conversations (30s measured) ──
turns          10576 ok, 0 failed   352.5 turns/s   peak 4270 in flight
first token    p50      618  p90     1167  p99     1977  max    12748  ms
added: ttft    p50    13.73  p90    37.05  p99    96.69  max    11978  ms
added: stream  p50     0.11  p90     1.04  p99     3.42  max     9.81  ms
after stream   p50     4.06  p90    10.93  p99    29.45  max    53.99  ms
token gap      p50    16.67  p90    20.94  p99    22.79  max    35.65  ms
whole turn     p50     7375  p90    10617  p99    12800  max    22147  ms
server         3.70 cores avg   727 MB peak RSS   8507 fds peak
loadgen        1.23 cores avg   195 MB peak RSS
               → pass
```

With an additional 16875 users, the `added: ttft` timeout skyrockets to over a second and
hits our configured acceptable limit of 250ms:
```
── step 5: 16875 concurrent conversations (30s measured) ──
turns          11566 ok, 0 failed   385.5 turns/s   peak 5451 in flight
first token    p50      662  p90     1251  p99     2218  max    14389  ms
added: ttft    p50    26.40  p90      195  p99     1225  max    14046  ms
added: stream  p50     0.08  p90     2.17  p99     8.54  max    37.22  ms
after stream   p50     5.08  p90    33.53  p99    75.71  max    92.61  ms
token gap      p50    16.76  p90    21.26  p99    25.81  max    62.02  ms
whole turn     p50     7422  p90    10715  p99    13003  max    24353  ms
server         4.41 cores avg   1873 MB peak RSS   9878 fds peak
loadgen        1.43 cores avg   676 MB peak RSS
               → fail: added ttft p99 1225ms (limit 250ms)
```
Note that the memory spike you see here is fake. This is step 5 of the benchmark, where it
already created 10000 users (30k convos), failed, then tried to scale down until it passed
again. This memory usage is for 30k convos, not 16k. This can be seen from the trace here:
```
step 1 (15k convos): 727 MB peak RSS
step 2 (30k convos): 1836 MB peak RS
step 3 (22.5k convos): 1854 MB peak RSS
step 4 (18.7k convos): 1865 MB peak RSS
step 5 (16.8k convos): 1873 MB peak RSS
```

## Final Note
This is admittedly on a high-end system. With real users and cheaper hardware, you can expect
the numbers to be a bit worse. A benchmark on a standard VPS will hopefully follow.