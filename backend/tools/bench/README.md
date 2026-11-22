# bench

Load tests for one Lorealm server, with the language model replaced by a fake
whose latency you control.

- **`mock-llm`**: a fake OpenAI-compatible `/v1/chat/completions` endpoint
  with configurable time to first token, token rate, length and faults.
- **`loadgen`**: creates bench users, points them at the mock, runs chat turns
  at increasing load and prints a report per step.

Results from a published run are in [`docs/benchmarks.md`](../../../docs/benchmarks.md).

## Running

All commands run from `backend/tools/bench`. Each target builds in release
mode, starts a fresh server and mock, runs loadgen, and shuts both down
afterwards, even on failure or Ctrl-C.

| Target | Does |
|---|---|
| `make quick` | Short run against an instant model, to check the setup |
| `make steps` | Measures each load level in `STEPS` |
| `make limit` | Finds the highest load that meets the pass criteria |
| `make up` / `make down` | Starts / stops the server and mock in the background |
| `make build` | Builds the server and the bench tools |
| `make clean` | Stops everything and deletes `WORK_DIR`, results included |

Results are written to `RESULTS_DIR` as timestamped JSON. Server and mock logs
are in `WORK_DIR` (`server.log`, `mock.log`).

## Variables

Set any of these on the command line, e.g. `make steps STEPS=100,500`.

### Load

| Variable | Default | Meaning |
|---|---|---|
| `MODE` | `closed` | `closed`: keep N conversations busy. `open`: start N turns per second |
| `STEPS` | `10,50,100` | Load levels for `make steps` |
| `START` | `50` | First load level for `make limit`; it doubles after each pass |
| `STEP_SECS` | `30` | Seconds measured per step |
| `WARMUP_SECS` | `5` | Unmeasured seconds before each step |
| `MODEL` | `mock` | Model name sent to the mock; can override mock settings (see below) |
| `TOKENIZER` | `hf:openai-community/gpt2` | Tokenizer installed for bench users |
| `CRITERIA` | | Pass criteria flags for `make limit` (see below) |
| `LOADGEN_ARGS` | | Any other loadgen flags (see below) |

### Mock

| Variable | Default | Meaning |
|---|---|---|
| `MOCK_ARGS` | `--ttft-ms 600 --tps 60 --min-tokens 200 --max-tokens 600` | mock-llm flags |
| `MOCK_ADDR` | `127.0.0.1:9100` | Where the mock listens |

### Server and storage

| Variable | Default | Meaning |
|---|---|---|
| `SERVER_BIN` | `../../target/release/lorealm_server` | Server binary to test, e.g. a copy of an older build |
| `SERVER_PORT` | `15510` | Server port |
| `SERVER_LOG` | `warn` | Server `RUST_LOG` |
| `POSTGRES_URL` | | Use Postgres instead of SQLite. Not wiped between runs, so give each run an empty database |
| `DB_CONNECTIONS` | 5 (SQLite), 10 (Postgres) | Database pool size. Postgres refuses more than its own `max_connections` |
| `REDIS_URL` | | Put a Redis cache in front of the database |
| `REDIS_TTL` | `300` | Redis entry lifetime in seconds |
| `WORK_DIR` | `../../target/bench` | Run state and logs; wiped on each run |
| `RESULTS_DIR` | `$(WORK_DIR)/results` | Where JSON results go |

## Common runs

### Throughput

Peak turns per second, with a model that answers instantly so the server is
the only cost:

```sh
make steps STEPS=50,200,500 MODEL='mock?ttft_ms=0&tps=0&tokens=200'
```

### Realistic limit

How many users one server carries. Each user has three conversations
streaming at once and pauses about 30 seconds (15–45) between turns to read
and type. The limit is the highest load where the server adds under 250 ms
(p99) before the first token and under 1% of turns fail:

```sh
dropdb --if-exists lorealm_bench && createdb lorealm_bench
make limit START=2000 \
  LOADGEN_ARGS='--think-ms 30000' \
  POSTGRES_URL=postgresql://localhost/lorealm_bench \
  REDIS_URL=redis://localhost \
  DB_CONNECTIONS=60
```

Load levels are concurrent conversations, so users = level / 3. Drop the
`POSTGRES_URL`, `REDIS_URL` and `DB_CONNECTIONS` lines to test SQLite.

### Fixed rate

Turns arriving at a steady rate whether or not earlier ones finished, so
queueing shows up in latency:

```sh
make steps MODE=open STEPS=50,100,200
```

### Large prompts

Cost of building and counting context for long character descriptions:

```sh
make steps STEPS=50,200 LOADGEN_ARGS='--character-bytes 50000'
```

### Flaky upstream

Error handling under load:

```sh
make steps STEPS=50,200 \
  MOCK_ARGS='--error-rate 0.05 --error-status 429 --stall-rate 0.02 --drop-rate 0.02'
```

### Comparing two builds

```sh
cp ../../target/release/lorealm_server /tmp/lorealm_server.before
# … make the change …
make steps STEPS=50,200 SERVER_BIN=/tmp/lorealm_server.before RESULTS_DIR=../../target/bench/results/before
make steps STEPS=50,200 RESULTS_DIR=../../target/bench/results/after
```

## Pass criteria

`make limit` passes a step when it meets every criterion that's switched on.
Set them through `CRITERIA`, e.g. `CRITERIA='--max-added-ttft-ms 100'`;
0 switches one off.

| Flag | Default | Fails when |
|---|---|---|
| `--max-error-rate` | `0.01` | More than this share of turns fail |
| `--max-added-ttft-ms` | `250` | p99 time the server adds before the first token is above this |
| `--max-token-gap-ms` | off | p99 gap between streamed tokens is above this |
| `--max-after-stream-ms` | off | p99 time from the last token to the turn finishing is above this |
| `--min-throughput-ratio` | `0.95` | Open mode: finished turns/s fall below this share of the arrival rate |

These also shape the search and can go in `CRITERIA` or `LOADGEN_ARGS`:

| Flag | Default | Meaning |
|---|---|---|
| `--growth` | `2` | Load multiplier after each passing step |
| `--refine` | `3` | Steps spent narrowing down after the first failure |
| `--max-level` | none | Highest load to try |

## Loadgen flags

Pass through `LOADGEN_ARGS`.

| Flag | Default | Meaning |
|---|---|---|
| `--think-ms` | `0` | Closed mode: average pause between a conversation's turns (each 0.5–1.5× this) |
| `--convs-per-user` | `3` | Conversations each bench user streams at once |
| `--reuse-conversations` | off | Keep conversations between steps, so history grows |
| `--character-bytes` | `2000` | Character description size, to grow the prompt |
| `--tokenizer-per-user` | off | Give each user a private copy of the tokenizer |
| `--turn-timeout-secs` | `120` | Give up on a turn after this long |
| `--seed` | clock | Seed for arrival times |
| `--max-open-files` | `max` | Soft open-file limit to raise to |

## Mock settings

Set defaults through `MOCK_ARGS` (`--ttft-ms 600`), or per run through the
model name (`MODEL='mock?ttft_ms=0&tps=0'`), which needs no mock restart.

| Flag / override | Default | Meaning |
|---|---|---|
| `--ttft-ms` / `ttft_ms` | `600` | Median time to first token |
| `--ttft-sigma` / `ttft_sigma` | `0.5` | Log-normal spread of time to first token; 0 makes it fixed |
| `--tps` / `tps` | `60` | Tokens per second; 0 streams as fast as possible |
| `--tps-jitter` / `tps_jitter` | `0.25` | Per-request token rate spread, as a fraction |
| `--min-tokens` / `min_tokens` | `200` | Fewest tokens per response |
| `--max-tokens` / `max_tokens` | `600` | Most tokens per response |
| `tokens` | | Sets both min and max |
| `--chunk-tokens` / `chunk_tokens` | `1` | Tokens per streamed chunk |
| `--error-rate` / `error_rate` | `0` | Share of requests answered with an error |
| `--error-status` / `error_status` | `500` | Status code for injected errors |
| `--stall-rate` / `stall_rate` | `0` | Share of streams that pause once partway |
| `--stall-ms` / `stall_ms` | `5000` | Length of an injected pause |
| `--drop-rate` / `drop_rate` | `0` | Share of streams cut off partway |

## Reading the report

| Row | Meaning |
|---|---|
| `turns` | Finished and failed turns, turns/s, peak turns in flight |
| `first token` | Request sent → first text |
| `added: ttft` | Time the server adds before the first token (mock delay subtracted) |
| `added: stream` | Time the server adds across the stream, first → last token |
| `after stream` | Last token → `done` (saving the message, etc.) |
| `token gap` | Time between text events |
| `whole turn` | Request sent → `done` |
| `server` / `loadgen` | Average cores, peak memory, peak open files |

The `added` rows are the server's own cost.

## Accurate numbers

- Run the mock and loadgen on other machines, or pin them to other cores
  (`taskset`). The `loadgen` row shows how much CPU the generator takes.
- Use a fresh database for every run you publish.
- If the server warns that the hard open-file limit capped it, raise it
  (`LimitNOFILE` under systemd, `ulimits` under Docker). The server setting is
  `hosting.max_open_files`.
- Above roughly 28k connections to the mock, run loadgen and mock-llm by hand
  with several addresses: `mock-llm --listen 127.0.0.1:9100 --listen 127.0.0.2:9100`
  and `loadgen --mock-url http://127.0.0.1:9100/v1 --mock-url http://127.0.0.2:9100/v1`.
- Publish the median of several runs, with hardware, backend and mock settings
  (all recorded in the JSON).
