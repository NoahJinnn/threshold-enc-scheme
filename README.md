# Threshold Encryption scheme

This document is an instruction for:

- Distributed Key Generation (DKG) workflow
- Threshold Encryption and Decryption (TED) workflow

This sample scheme involves 2 nodes `client` and `server`, both is written in Rust and implemented as HTTP server using `axum`.

Restful API is the main protocol we used here for the communication between 2 nodes in the demo.

## Prerequisite

- Install Rust toolchains. Follow is mine

```sh
rustup 1.25.2 (17db695f1 2023-02-01)
rustc 1.69.0 (84c898d65 2023-04-16)
cargo 1.69.0 (6e9a83356 2023-04-12)
```

## DKG

This scheme has 3 routes:

1 /init: 
- req: 
  - server receive client publickey `p1_pk`
- server exec:
  - generate keypair
  - create `sync_key_gen_0` instance
  - create server part `p0_part`
- resp
  - send back server publickey `p0_pk` + server part `p0_part` to client so that client can create its `sync_key_gen`

=> `server`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_0`, `p0_part`
=> `client`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_1`, `p0_part`, `p1_part`

2 /commit: 
- req:
  - server receive from client: `p1_part`, `p1_pk` and `p1_acks` list (client will run "acknowledge process" with its part `p1_part` + server part `p0_part` to validate parts and generate its `p1_acks` list)
- server exec:
    - run "acknowledge process" with its part `p0_part` + client part `p1_part` to validate parts and generate its `p0_acks` list as a result.
- resp
  - send its `p0_acks` list to client. Now both parties has all `ack` result of communication

=> `server`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_0`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`
=> `client`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_1`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`

3 /finalize:
- req:
  - client able to use its `sync_key_gen_1` instance to generate `pubkey_set` -> create`pubkey_share` `pks1` and send to server
- server exec:
  - use `sync_key_gen_0` instance to generate `pubkey_set`: `pubkey_share` `pks0` and `secret_key_share` `sks0`
  - sign message with `sks0`
  - verify the signed message with `pks1` from client
- resp:
  -  send back the status of verifying success or fail

=> `server`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_0`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`, `signed message`, `pks0`, `sks0` 
=> `client`: a keypair, `p1_pk`, `p0_pk`, `sync_key_gen_1`, `p0_part`, `p1_part`, `p0_acks`, `p1_acks`, `signed message`, `pks1`, `sks1`

Usage:

Clone this repository

```sh
cd server
cargo run # Server currently running on port 3000
```

```sh
cd client
cargo run # Server currently running on port 3001
```

Call 3 route sequencely:

```sh
curl --location --request POST 'localhost:3001/init_dkg'
curl --location --request POST 'localhost:3001/commit'
curl --location --request POST 'localhost:3001/finalize_dkg'
```

```sh
# client
2023-06-24T05:02:34.844803Z DEBUG client: listening on 127.0.0.1:3001
2023-06-24T05:02:44.888717Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-24T05:02:44.888812Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-24T05:02:44.889246Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2023-06-24T05:02:44.912130Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-24T05:02:44.912347Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-24T05:02:44.912948Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-24T05:02:44.913632Z DEBUG hyper::proto::h1::io: flushed 300 bytes
2023-06-24T05:02:45.065253Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-24T05:02:45.065277Z DEBUG hyper::proto::h1::conn: incoming body is content-length (1741 bytes)
2023-06-24T05:02:45.065326Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:45.065439Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
2023-06-24T05:02:45.185844Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=296 ms status=200
2023-06-24T05:02:45.186004Z DEBUG hyper::proto::h1::io: flushed 111 bytes
2023-06-24T05:02:47.201196Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-24T05:02:47.201258Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-24T05:02:47.201484Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_request: started processing request
Node #1 handles Part from node success #0
Node #1 handles Part from node success #1
2023-06-24T05:02:47.739550Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-24T05:02:47.739625Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-24T05:02:47.740054Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-24T05:02:47.740224Z DEBUG hyper::proto::h1::io: flushed 4316 bytes
2023-06-24T05:02:48.256026Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-24T05:02:48.256052Z DEBUG hyper::proto::h1::conn: incoming body is content-length (5236 bytes)
2023-06-24T05:02:48.256085Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:48.256194Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
2023-06-24T05:02:48.256544Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=1055 ms status=200
2023-06-24T05:02:48.256623Z DEBUG hyper::proto::h1::io: flushed 111 bytes
2023-06-24T05:02:49.580320Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-24T05:02:49.580391Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-24T05:02:49.580654Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2023-06-24T05:02:49.908627Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-24T05:02:49.908700Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-24T05:02:49.910677Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-24T05:02:49.910864Z DEBUG hyper::proto::h1::io: flushed 301 bytes
2023-06-24T05:02:50.495579Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-24T05:02:50.495601Z DEBUG hyper::proto::h1::conn: incoming body is content-length (19 bytes)
2023-06-24T05:02:50.495630Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:50.495727Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
is_success: FinalizeResp { is_success: true }
2023-06-24T05:02:50.495866Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=915 ms status=200
2023-06-24T05:02:50.495941Z DEBUG hyper::proto::h1::io: flushed 111 bytes
2023-06-24T05:02:58.032943Z DEBUG hyper::proto::h1::io: parsed 8 headers
2023-06-24T05:02:58.033003Z DEBUG hyper::proto::h1::conn: incoming body is empty
2023-06-24T05:02:58.033215Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
2023-06-24T05:02:58.035190Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: reqwest::connect: starting new connection: http://127.0.0.1:3000/    
2023-06-24T05:02:58.035329Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::connect::http: connecting to 127.0.0.1:3000
2023-06-24T05:02:58.035792Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::connect::http: connected to 127.0.0.1:3000
2023-06-24T05:02:58.036238Z DEBUG hyper::proto::h1::io: flushed 301 bytes
2023-06-24T05:02:58.340044Z DEBUG hyper::proto::h1::io: parsed 3 headers
2023-06-24T05:02:58.340064Z DEBUG hyper::proto::h1::conn: incoming body is content-length (19 bytes)
2023-06-24T05:02:58.340088Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:58.340165Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: hyper::client::pool: pooling idle connection for ("http", 127.0.0.1:3000)
is_success: FinalizeResp { is_success: true }
2023-06-24T05:02:58.340294Z DEBUG request{method=POST uri=/finalize_dkg ver
```

```sh
# server
2023-06-24T05:02:38.427581Z DEBUG server: listening on 127.0.0.1:3000
2023-06-24T05:02:44.914195Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-24T05:02:44.914290Z DEBUG hyper::proto::h1::conn: incoming body is content-length (185 bytes)
2023-06-24T05:02:44.914434Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:44.915394Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body InitDkgReq { p1_pk: PublicKey(1984..703d) }
2023-06-24T05:02:45.064890Z DEBUG request{method=POST uri=/init_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=150 ms status=200
2023-06-24T05:02:45.065049Z DEBUG hyper::proto::h1::io: flushed 1851 bytes
2023-06-24T05:02:45.073888Z DEBUG hyper::proto::h1::conn: read eof
2023-06-24T05:02:47.740372Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-24T05:02:47.740392Z DEBUG hyper::proto::h1::conn: incoming body is content-length (4202 bytes)
2023-06-24T05:02:47.740450Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:47.740512Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body CommitReq { p1_part: Part("<degree 0>", "<2 rows>"), p1_acks: [Ack(0, "<2 values>"), Ack(1, "<2 values>")] }
2023-06-24T05:02:48.255510Z DEBUG request{method=POST uri=/commit version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=514 ms status=200
2023-06-24T05:02:48.255744Z DEBUG hyper::proto::h1::io: flushed 5346 bytes
2023-06-24T05:02:48.256584Z DEBUG hyper::proto::h1::conn: read eof
2023-06-24T05:02:49.913139Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-24T05:02:49.913170Z DEBUG hyper::proto::h1::conn: incoming body is content-length (182 bytes)
2023-06-24T05:02:49.913226Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:49.913281Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body FinalizeReq { pks_1: PublicKeyShare(07e5..2bf7) }
is_success true
2023-06-24T05:02:50.495282Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=582 ms status=200
2023-06-24T05:02:50.495416Z DEBUG hyper::proto::h1::io: flushed 127 bytes
2023-06-24T05:02:50.496013Z DEBUG hyper::proto::h1::conn: read eof
2023-06-24T05:02:58.036423Z DEBUG hyper::proto::h1::io: parsed 4 headers
2023-06-24T05:02:58.036459Z DEBUG hyper::proto::h1::conn: incoming body is content-length (182 bytes)
2023-06-24T05:02:58.036545Z DEBUG hyper::proto::h1::conn: incoming body completed
2023-06-24T05:02:58.036699Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_request: started processing request
req_body FinalizeReq { pks_1: PublicKeyShare(07e5..2bf7) }
is_success true
2023-06-24T05:02:58.339734Z DEBUG request{method=POST uri=/finalize_dkg version=HTTP/1.1}: tower_http::trace::on_response: finished processing request latency=303 ms status=200
2023-06-24T05:02:58.339854Z DEBUG hyper::proto::h1::io: flushed 127 bytes
2023-06-24T05:02:58.340439Z DEBUG hyper::proto::h1::conn: read eof
```