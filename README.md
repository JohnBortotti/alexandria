# Alexandria

## about

raft based distributed database from scratch

## todo

- [ ] finish and test raft implementation
    - [x] raft startup (follower and candidate implementation)
    - [ ] leader node (log replication, log commit, state_machine)
- [ ] write doc about the threading model of a peer
- [ ] configure log level (production, debug, channel, etc)
- [ ] store-db engine
- [ ] review Messages struct (i guess i can remove the field "term" since every message already contains this info)
