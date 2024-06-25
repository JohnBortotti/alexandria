![The School of Athens](https://gcdnb.pbrd.co/images/4CoyyLWfkAKa.png?o=1)

# Alexandria

## description

**Alexandria** is a raft-based distributed key-value database from scratch.

## usage

1. check configuration at `config.yaml`
2. docker compose up
3. send requests to any peer

available commands:
- list collections: `list`
- create collection: `create {collection_name}`
- get entry: `get {collection_name} {key}`
- create entry: `write {collection_name} {key} {value}`
- delete entry: `delete {collection_name} {key}`

#### example

1. docker compose up
2. send request: (use -L flag to follow redirects, since followers nodes redirects 
some requests to leader)
    - `curl -L -X POST -d "get test a" 192.30.101:5000 && echo`

## todo

- [ ] implement bloom filter on storage
- [ ] update storage underlying structure
- [ ] implement sstable compression
