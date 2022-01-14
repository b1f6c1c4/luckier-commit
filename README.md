# luckier-commit

Make your git commits luckier every time, counting from `0000000`, `0000001`, `0000002`, ...!

## What?

With this simple tool, you can change the start of your git commit hashes to whatever you want, by default to be `HEAD^ + 0x1`.

```bash
$ git init
$ git luckier-commit --allow-empty -m 'Init commit'
$ git luckier-commit --allow-empty -m 'Another commit'
$ git luckier-commit --allow-empty -m 'One more commit'
$ git log
0000000 Init commit
0000001 Another commit
0000002 One more commit
```

As a demonstration, see the latest commits in this repository.

## How?

`luckier-commit` amends your commit messages by adding a few characters of various types of whitespace, and keeps trying new messages until it finds a good hash. By default, it will keep searching until it finds a hash starting with one plus the hash of its first-parent, but this can be changed by simply passing the desired hash as an argument.

```bash
$ luckier_commit 1010101
$ git log
1010101 Some commit
```

## Why?

¯\\\_(ツ)\_/¯

## Installation

You can build from source:

```
$ git clone https://github.com/b1f6c1c4/luckier-commit
$ cd luckier-commit/
$ cargo build --release
```

This will create the `luckier_commit` binary (`luckier_commit.exe` on Windows) in the `target/release` directory. You can move this to wherever you want, or set up an alias for it.

### Troubleshooting linker errors

By default, `luckier-commit` links with your system's OpenCL headers and runs on a GPU. This makes it significantly faster.

However, if you encounter a linker error along the lines of `/usr/bin/ld: cannot find -lOpenCL`, there are a few workarounds:

* Compile `luckier-commit` without OpenCL by adding the flag `--no-default-features` to your install or build command (i.e. `cargo install luckier_commit --locked --no-default-features` or `cargo build --release --no-default-features`). This will make `luckier-commit` fall back to a multithreaded CPU implementation. The CPU implementation is about 10x slower on my laptop, but depending on what you're planning to use the tool for, there's a good chance it's fast enough anyway.

    This is the recommended approach if you just want a stable build, and you don't need the extra performance from GPUs.
* You can try installing the OpenCL libraries for your system. The instructions for this will vary by OS (see e.g. [here](https://software.intel.com/content/www/us/en/develop/articles/opencl-drivers.html)). Note that this will only be useful if your machine has a GPU.

## Performance

`luckier-commit`'s performance is determined by how powerful your computer is, whether you GPG-sign your commits, and whether you use experimental git features.

### Hash rate

The main bottleneck is SHA1 throughput. The default hash prefix of (usually) has length 7, so on average, `luckier-commit` needs to compute 16<sup>7</sup> SHA1 hashes.

For non-GPG-signed commits, `luckier-commit` adds its whitespace to a 64-byte-aligned block at the very end of the commit message. Since everything that precedes the whitespace is constant for any particular commit, this allows `luckier-commit` to cache the SHA1 buffer state and only hash a single 64-byte block on each attempt.

Hash searching is extremely parallelizable, and `luckier-commit` takes advantage of this by running on a GPU when available. (The intuitive idea is that if you pretend that your commits are actually graphical image data, where SHA1 is a "shading" that gets applied to the whole image at once, and the resulting commit shorthashes are, say, RGBA pixel color values, then you can hash a large number of commits at once by just "rendering the image".)

It takes me **0.62 seconds** on average to find a 7-digit commit hash on my desktop (16-core AMD Ryzen 9 5950X). You can estimate the average time for your computer by running `time luckier_commit --benchmark`.

### GPG signatures

For GPG-signed commits, the commit message is part of the signed payload, so `luckier-commit` can't edit the commit message without making the signature invalid. Instead, it adds its whitespace to the end of the signature itself. Since the signature precedes the commit message in git's commit encoding, this requires `luckier-commit` to do more work on each attempt (it can't cache the SHA1 buffer state as effectively, and it needs to rehash the commit message every time). As a result, the performance for GPG-signed commits depends on the length of the commit message. This multiplies the average search time by roughly `1 + ceiling(commit message length / 64 bytes)`.

### SHA256 repositories

Finally, `luckier-commit` also supports git repositories using the [experimental sha256 object format](https://git-scm.com/docs/hash-function-transition/). If `luckier-commit` detects that it's being run in a repository with sha256 objects, it will automatically customize the sha256 shorthash of the commit at `HEAD`, rather than the sha1 shorthash. The hash rate for sha256 is a bit slower than the hash rate for sha1.

If you're wondering whether your repository uses sha256, then it probably doesn't. At the time of writing, this is a highly experimental feature and is very rarely used.

## Related projects

* [`lucky-commit`](https://github.com/not-an-aardvark/lucky-commit)
