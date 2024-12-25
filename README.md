# bitcoin inscription scanner

this scans the bitcoin blockchain for inscriptions. it finds both text and images. it's built for high performance with caching, resuming where it left off, and parallel processing.

## what it does

- scans blocks in parallel using rayon
- detects inscriptions efficiently
- handles text and image inscriptions
- caches data to disk
- can resume from last position
- connects to bitcoin nodes concurrently
- logs everything important

## what you need

- rust 1.56 or newer
- bitcoin core node
- leveldb

## how to install

1. clone it:
```bash
git clone https://github.com/yourusername/bitcoin-inscription-scanner.git
cd bitcoin-inscription-scanner
```

2. build it:
```bash
cargo build --release
```

## how to configure

make a config.toml file like this:

```toml
[node]
rpc_url = "http://127.0.0.1:8332"
rpc_user = "your_user"
rpc_password = "your_password"
max_concurrent_requests = 16

[storage]
image_dir = "./data/images"
text_log = "./data/inscriptions.log"

[cache]
enabled = true
path = "./data/cache"
bloom_filter_size = 1000000
bloom_filter_fp_rate = 0.01

[processing]
parallel_blocks = 8
batch_size = 1000
```

## how to use it

basic usage:
```bash
# start from the beginning
./target/release/bitcoin-inscription-scanner

# start at a specific block
./target/release/bitcoin-inscription-scanner --start-block 780000

# resume from where you left off
./target/release/bitcoin-inscription-scanner --resume

# test without a bitcoin node
./target/release/bitcoin-inscription-scanner --mock
```

you can also use environment variables:
```bash
export BIS_RPC_USER="your_user"
export BIS_RPC_PASSWORD="your_password"
export BIS_RPC_URL="http://127.0.0.1:8332"
```

## how it's built

```
src/
├── main.rs                 # where it starts
├── config/                 # handles settings
├── node/                   # talks to bitcoin
├── parser/                # finds inscriptions
├── storage/               # saves the data
└── utils/                 # helper stuff
```

## performance details

makes things fast by:
1. processing blocks in parallel with rayon
2. managing memory carefully
3. pooling rpc connections
4. using bloom filters for lookups
5. batching database writes

## how to contribute

1. fork it
2. make a branch
3. write tests
4. make sure tests pass
5. send a pull request

## testing

run all tests:
```bash
cargo test
```

run specific tests:
```bash
cargo test --features="test-integration"
```

## license

mit license - see license file

## security stuff

1. always check rpc credentials
2. use secure connections to remote nodes
3. validate inscription data
4. watch resource usage
5. keep dependencies updated

## technical details

### inscription detection
```rust
fn parse_script(&self, script: &Script) -> Option<InscriptionType> {
    let mut instructions = script.instructions().peekable();
    
    match (instructions.next()?, instructions.next()?) {
        (Ok(first), Ok(Instruction::Op(op2))) => {
            let is_false = match first {
                Instruction::Op(op1) => op1 == OP_FALSE || op1 == OP_0,
                Instruction::PushBytes(data) => data.as_bytes().is_empty(),
                _ => false,
            };

            if is_false && op2 == OP_IF {
                self.parse_inscription_content(&mut instructions)
            } else {
                None
            }
        }
        _ => None,
    }
}
```

### parallel processing
```rust
pub fn process_blocks(&self, blocks: Vec<Block>) -> Vec<Inscription> {
    blocks
        .par_iter()
        .flat_map(|block| {
            block.txdata
                .par_iter()
                .filter_map(|tx| self.parse_transaction(tx))
                .collect::<Vec<_>>()
        })
        .collect()
}
```

### caching system
```rust
pub struct MultiLevelCache {
    memory_cache: MemoryCache,
    disk_cache: DiskCache,
    bloom_filter: BloomFilter,
}
```

## acknowledgments

- bitcoin core devs
- rust bitcoin community
- ordinals protocol devs