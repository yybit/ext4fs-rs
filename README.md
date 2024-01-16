[![crates.io](https://img.shields.io/crates/v/ext4fs.svg)](https://crates.io/crates/ext4fs)
[![docs.rs](https://docs.rs/ext4fs/badge.svg)](https://docs.rs/ext4fs)

## ext4fs-rs

Rust implementation of ext4 file system in user space. The ext4 file system can be read directly without mounting, write operations are not supported yet. Your contributions are welcome.

:warning::warning::warning: The current api is not stable, it may be modified later, and more tests and documentation will need to be added.


### Example

* New File System

```rust
// Read a raw ext4 image file.
let file = std::fs::File::open("testdata/test.ext4").unwrap();
let reader = BufReader::new(file);
let mut fs = ext4fs::FileSystem::from_reader(reader).unwrap();
```

* Iterate a directory

```rust
let rd = fs.read_dir("/dir1").unwrap();
for x in rd {
    println!("{}", x.unwrap().get_name_str());
}
```

* Stat a file

```rust
let m = fs.metadata("/hello.txt").unwrap();
println!(
    "uid={} gid={} permissions={:o} len={} created={:?} accessed={:?} modified={:?}",
    m.uid(),
    m.gid(),
    m.permissions(),
    m.len(),
    m.created(),
    m.accessed(),
    m.modified(),
);
```

* Read symlink

```rust
let p = fs.read_link("/hello.txt.lnk").unwrap();
println!("{}", p.to_str().unwrap());
```

* Read all contents of file

```rust
let b = fs.read("/hello.txt").unwrap();
assert_eq!("hello\n", String::from_utf8_lossy(&b).to_string());
```

* Read file on demand

```rust
let mut f = fs.open("/hello.txt").unwrap();
f.seek(std::io::SeekFrom::Start(2)).unwrap();
let mut buf = String::new();
f.read_to_string(&mut buf).unwrap();
```