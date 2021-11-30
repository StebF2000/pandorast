# Pandorast

## Objectives
1. Improve [Pandora](https://github.com/HPC4SC/PANDORA) performance using Rustlang.
2. Try different memory allocators (jemallocator, mimalloc) if system supported.
3. Auto-vectorization
4. Profile-guided optimization using llvm
5. Going deep to assembly-like code if needed (not wished).
6. Create a good Bachelor thesis
7. Make good OpenSource code

## Resources
- Build Rust Docker containers efficiently
  - https://www.lpalmieri.com/posts/fast-rust-docker-builds/

- Concurrency with Rustlang
  - https://www.youtube.com/watch?v=Dbytx0ivH7Q&t=2493s
  
- Increase performance on Rust
  - https://deterministic.space/high-performance-rust.html
  - https://www.nickwilcox.com/blog/autovec/

- Profile-guided optimization
  - https://doc.rust-lang.org/1.41.1/rustc/profile-guided-optimization.html

- How to write fast code on Rust
  - https://likebike.com/posts/How_To_Write_Fast_Rust_Code.html
  - https://gist.github.com/jFransham/369a86eff00e5f280ed25121454acec1#number-one-optimization-tip-dont

- Profiling Rust
  - https://blog.anp.lol/rust/2016/07/24/profiling-rust-perf-flamegraph/