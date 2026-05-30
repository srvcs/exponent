# srvcs-exponent

Arithmetic microservice for srvcs.cloud: raises a `base` to an `exp` (real
exponentiation). The result is an `f64` (a JSON number that may be fractional).

This service is an **orchestrator**: it owns the control flow but delegates the
arithmetic to [`srvcs-floatpower`](https://github.com/srvcs/floatpower). It does
not validate inputs itself — validation propagates from its dependency (it
forwards `422`s).

## Identity

`GET /` returns:

```json
{
  "service": "srvcs-exponent",
  "concern": "arithmetic: base raised to exp (real)",
  "depends_on": ["srvcs-floatpower"]
}
```

## Evaluate

`POST /` with `{"base": <number>, "exp": <number>}` returns
`{"base", "exp", "result": <float>}`.

```sh
curl -s localhost:8080/ -d '{"base": 2, "exp": 10}'
# {"base":2.0,"exp":10.0,"result":1024.0}
curl -s localhost:8080/ -d '{"base": 9, "exp": 0.5}'
# {"base":9.0,"exp":0.5,"result":3.0}
```

### Algorithm

```text
result (f64) = (call srvcs-floatpower {"base": base, "exp": exp}).result
```

So `exponent(2, 10) == 1024.0` and `exponent(9, 0.5) == 3.0`.

### Responses

- `200` — `{"base", "exp", "result"}` on success.
- `422` — forwarded from `srvcs-floatpower` when it rejects the input.
- `500` — `srvcs-floatpower` answered `200` but with a malformed result.
- `503` — `srvcs-floatpower` is unreachable (degraded).

## Configuration

| Variable                | Default                 | Purpose                          |
| ----------------------- | ----------------------- | -------------------------------- |
| `SRVCS_BIND_ADDR`       | `0.0.0.0:8080`          | Listen address.                  |
| `SRVCS_FLOATPOWER_URL`  | `http://127.0.0.1:8090` | Base URL of `srvcs-floatpower`.  |
| `RUST_LOG`              | `info,tower_http=info`  | Log filter.                      |
| `SRVCS_ENV`             | `development`           | Environment label.               |

## Local checks

```sh
nix flake check -L
nix develop -c sh -euc 'cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test'
nix build .#default -L
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared service
standard and CI workflow.
