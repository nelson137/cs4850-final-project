# Chat Boat

```
  .oooooo.   oooo                      .
 d8P'  `Y8b  `888                    .o8
888           888 .oo.    .oooo.   .o888oo
888           888P"Y88b  `P  )88b    888
888           888   888   .oP"888    888
`88b    ooo   888   888  d8(  888    888 .
 `Y8bood8P'  o888o o888o `Y888""8o   "888"

oooooooooo.                            .
`888'   `Y8b                         .o8
 888     888   .ooooo.    .oooo.   .o888oo
 888oooo888'  d88' `88b  `P  )88b    888
 888    `88b  888   888   .oP"888    888
 888    .88P  888   888  d8(  888    888 .
o888bood8P'   `Y8bod8P'  `Y888""8o   "888"

                     __/___
               _____/______|
       _______/_____\_______\_____
       \              < < <       |
     ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
```

## How to Run

This project is written in [Rust](https://www.rust-lang.org/). The minimum supported version is 1.49 [due to one of the dependencies](https://docs.rs/tracing/latest/tracing/#supported-rust-versions), however, it is recommended to run with the lastest version, 1.59. This is only an issue if Rust is already installed on your machine and has not been updated in over a year (version 1.49 was released on December 31, 2020). If so, please run `rustc --version` to verify.

The project can either be run locally (which requires the rust toolchain to be installed) or with docker. See below for guides.

### Build & run manually

<details>
<summary>Click to expand</summary>

<br />
Rust is setup with Rustup, the official installer. If not already installed, please visit <a href="https://rustup.rs/#">the Rustup website</a> and follow the steps there before continuing.
<br /><br />
Use Cargo, the package manager that comes with Rust, to build and run.
<br /><br />
To run the server:
<pre>
$ cargo run --release --bin chat-server
</pre>
To run the client:
<pre>
$ cargo run --release --bin chat-client
</pre>

</details>

### Build & run with docker

<details>
<summary>Click to expand</summary>

<br />
All docker-related files are located in the <code>docker</code> directory in the project root. Be sure to <code>cd</code> into this directory first.
<br /><br />
A makefile is provided that will build and run the server and client containers for you. Note that the first time a container is run it will likely take between 60 and 90 seconds to download the dependencies and build the shared library. After that, containers should either start immediately, or take a few seconds if that image hasn't been built yet since it must also compile the binary.
<br /><br />
To run a server container:
<pre>
$ make server
</pre>
To run a client container:
<pre>
$ make client
</pre>

</details>

## Code Structure

```
📁 src
├─ 📁 libchat          (shared library)
│  ├─ 📁 sys           (syscall wrappers and helpers)
│  │  └─ ...
│  ├─ 📄 lib.rs        (library entry point)
│  ├─ 📄 banner.rs     (banner graphics)
│  ├─ 📄 err.rs        (custom error type)
│  ├─ 📄 signal.rs     (utilities for registering signal handlers)
│  └─ 📄 users_dao.rs  (model for the users database)
├─ 📁 chat-client      (client binary)
│  ├─ 📄 main.rs       (binary entry point)
│  ├─ 📄 repl.rs       (CLI REPL)
│  └─ 📄 client.rs     (specialized socket wrapper)
└─ 📁 chat-server      (server binary)
   ├─ 📄 main.rs       (binary entry point)
   └─ 📄 server.rs     (specialized socket wrapper)
```
