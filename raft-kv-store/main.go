// Copyright 2015 The etcd Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package main

import (
	"flag"
	"strings"

	log "github.com/sirupsen/logrus"
	"go.etcd.io/etcd/raft/v3/raftpb"
)

func main() {
	cluster := flag.String("cluster", "http://172.31.2.227:10000,"+
		"http://172.31.8.130:11000,http://172.31.1.13:12000",
		"comma separated cluster peers")
	id := flag.Int("id", 1, "node ID")
	kvport := flag.Int("port", 9121, "key-value server port")
	join := flag.Bool("join", false, "join an existing cluster")
	debug := flag.Bool("d", false, "enable debug logging")
	snapshotCount := flag.Uint64("count", 10000, "snapshot count")
	baseSnapPath := flag.String("s", "/tmp/", "snapshot path")
	flag.Parse()

	if *debug {
		log.SetLevel(log.InfoLevel)
	} else {
		log.SetLevel(log.ErrorLevel)
	}

	proposeC := make(chan string)
	defer close(proposeC)
	confChangeC := make(chan raftpb.ConfChange)
	defer close(confChangeC)

	// raft provides a commit stream for the proposals from the http api
	var kvs *kvstore
	getSnapshot := func() ([]byte, error) { return kvs.getSnapshot() }
	commitC, errorC, snapshotterReady := newRaftNode(*id,
		strings.Split(*cluster, ","), *join, getSnapshot, proposeC,
		confChangeC, *debug, *snapshotCount, *baseSnapPath)

	kvs = newKVStore(<-snapshotterReady, proposeC, commitC, errorC)

	// the key-value http handler will propose updates to raft
	serveHttpKVAPI(kvs, *kvport, confChangeC, errorC)
}
