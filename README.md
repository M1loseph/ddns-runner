# DDNS Runner

An app that calls [Duck DNS](https://www.duckdns.org/) at customizable interval to update DNS record.

## Running on production

There is a handy [Docker image](https://hub.docker.com/r/m1loseph/ddns-runner). Set the following environment variables:

- `DDNS_RUNNER_DOMAINS_TO_UPDATE` - the domains which ip you want to update. They should be seperated by a semicolumn (`;`).
- `DDNS_RUNNER_TOKEN` - token that is used by the app to authorize to Duck DNS. It can be obtained on [https://www.duckdns.org/](https://www.duckdns.org/) (login first).
- `RUST_LOG` - logging level of the application.

Optional configuration:

- `DDNS_RUNNER_IP_UPDATE_INTERVAL_SEC` - ip update interval in seconds. Defaults to 300 (5 minutes).
- `DDNS_RUNNER_SERVER_PORT` - port for the server that hosts prometheus metrics on `/internal/status/prometheus` endpoint. Defaults to `3000`.

## Running locally

First make sure to install packages required by this project. These are:

- rust (via [https://www.rust-lang.org/learn/get-started](https://www.rust-lang.org/learn/get-started))
- libssl-dev (via `sudo apt install libssl-dev`)
- build-essential (via `sudo apt install build-essential`)

### With actual Duck DNS

Set the environments variables as described in [Running on production](#running-on-production) and run the app using `cargo run` command.
The app supports `.env` file, so instead of setting environment variables manually in the terminal, you can just create `.env` file and put them all there.

### With mock server

Copy file `.env-dev` and name it `.env`. Run project `duck-dns-server-mock` in another terminal and after that run `ddns-runner`.

## Prometheus metrics

You can access prometheus metrics on `/internal/status/prometheus` endpoint. Below there is a table of exposed metrics.

| Metric name | Description | Tags |
| --- | --- | --- |
| `ddnsrunner_ddns_requests` | Number of requests made to Duck DNS | `result`:  `failure`, `success` |

## Building the docker image

`docker build -t <image-name> .`

`docker push <image-name>`
