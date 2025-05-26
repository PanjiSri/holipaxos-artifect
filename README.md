# holipaxos-artifect

## Directory
1. [holipaxos](https://github.com/Zhiying12/holipaxos): The HoliPaxos implementation. Its instruction is inside the folder.
2. multipaxos: The MultiPaxos implementation.
3. raft-kv-store: The etcd's Raft implementation.
4. Copilot: The Copilot Paxos implementation.
5. omnipaxos-kv-store: The OmniPaxos KV store implementation.
6. ycsb: The YCSB benchmark close-loop client.
7. ycsb-openloop: The updated open-loop client on top of original YCSB benchmark.

## Installation
Please skip this part if you are using the provided AWS AMI instance, as environment and scriptes are set on the instance.

### Prerequisites
This artifect needs [rocksdb](https://github.com/facebook/rocksdb/blob/master/INSTALL.md), [protobuf](https://grpc.io/docs/protoc-installation/), [maven](https://maven.apache.org/install.html), [docker](https://docs.docker.com/engine/install/), [go](https://go.dev/doc/install), and [rust](https://www.rust-lang.org/tools/install).

### Install
For HoliPaxos:
```
pushd holipaxos
go build -o bin/replicant main/main.go
docker build -t holipaxos .
popd
```
For MultiPaxos:
```
pushd multipaxos
go build -o bin/replicant main/main.go
docker build -t multipaxos .
popd
```
For raft and raft+:
```
pushd raft-kv-store
go build -o bin/replicant main.go
popd
```
For Copilot:
```
pushd copilot
go build -o bin/master master/master.go
go build -o bin/server master/server.go
docker build -t copilot .
popd
```
For OmniPaxos:
```
pushd omnipaxos-kv-store
cargo build --release
popd
```


## Run
The entire experiment requires at 6 instances for servers and clients and 1 additional instance serving as experiment controller, which is optional. The required 6 instances must be m5.2xlarge size (8 CPU and 32 GB RAM), and the rest one that serves as a experiment controller can be a smaller m5 instance.

Replace the IP address for each machine in the 3-node.json and 5-nodee.json repectively. Then run the script, which will broadcast the json file to all machines and start experiments.

In each version of implementation, they all have corresponding scripts to start servers. The script for HoliPaxos, MultiPaxos, Raft, and OmniPaxos is run.sh, where the command format is `./run [node_id]`. As Copilot Paxos needs a master for a single cluster, it runs `./run-master.sh` on one of the machines in cluster and runs `./run-server.sh` on the rest machines.

As for the YCSB benchmark, the easist way to use YCSB clients is to use the script under this path, ycsb/scripts/throughput-latency/run.sh. The command format is shown as follows. `./scripts/throughput-latency/run.sh [DB_NAME] [CLIENT_COUNT] [KEY_VALUE_PAIR_COUNT_IN_DB] [NAME_of_EXPERIMENT]`. Similar to servers, "multipaxos" is used as DB_NAME for HoliPaxos, MultiPaxos, Raft, and OmniPaxos, while "copilot" is used for Copilot Paxos.

For convenience, the python script cover all the parts of triggering each single script. To make it work, all the IP addresses should be updated in the ip_list.txt file. The first line would be reserved for the client machine, and the rest order of IP addresses represent the order of machine IDs. Additionally, the pem file should be copied along with the controller machine, so the controller machine can replicate files and issue commands.

