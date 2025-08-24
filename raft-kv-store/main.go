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
	"encoding/json"
	"flag"
	"os"
	"strings"

	log "github.com/sirupsen/logrus"
	"go.etcd.io/etcd/raft/v3/raftpb"
)

type Config struct {
	ThreadpoolSize   int      `json:"threadpool_size"`
	CommitInterval   int      `json:"commit_interval"`
	Peers            []string `json:"peers"`
	Store            string   `json:"store"`
	DBPath           string   `json:"db_path"`
	CheckQuorum      bool     `json:"check_quorum"`
	SnapshotInterval int      `json:"snapshot_interval"`
}

func main() {
	cluster := flag.String("cluster", "",
		"comma separated cluster peers")
	configFile := flag.String("config", "", "path to config JSON file")
	id := flag.Int("id", 1, "node ID")
	kvport := flag.Int("port", 9121, "key-value server port")
	join := flag.Bool("join", false, "join an existing cluster")
	debug := flag.Bool("d", false, "enable debug logging")
	snapshotCount := flag.Uint64("count", 10000, "snapshot count")
	baseSnapPath := flag.String("s", "/tmp/", "snapshot path")
	flag.Parse()

	var clusterPeers []string
	
	if *configFile != "" {
		configData, err := os.ReadFile(*configFile)
		if err != nil {
			log.Fatalf("Failed to read config file: %v", err)
		}
		
		var config Config
		if err := json.Unmarshal(configData, &config); err != nil {
			log.Fatalf("Failed to parse config file: %v", err)
		}
		
		for _, peer := range config.Peers {
			if strings.HasPrefix(peer, "0.0.0.0:") {
				peer = strings.Replace(peer, "0.0.0.0:", "127.0.0.1:", 1)
			}
			clusterPeers = append(clusterPeers, "http://"+peer)
		}
	} else if *cluster != "" {
		clusterPeers = strings.Split(*cluster, ",")
	} else {
		clusterPeers = []string{
			"http://127.0.0.1:10000",
			"http://127.0.0.1:11000", 
			"http://127.0.0.1:12000",
		}
	}

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
		clusterPeers, *join, getSnapshot, proposeC,
		confChangeC, *debug, *snapshotCount, *baseSnapPath)

	kvs = newKVStore(<-snapshotterReady, proposeC, commitC, errorC)

	// the key-value http handler will propose updates to raft
	serveHttpKVAPI(kvs, *kvport, confChangeC, errorC)
}
